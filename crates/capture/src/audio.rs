use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use crossbeam_channel::Sender;
use greyscreen_core::media::{AudioTimeline, Span};
use greyscreen_core::time::{Hns, mono_now};
use wasapi::{
    AudioCaptureClient, AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode,
    WasapiError, WaveFormat,
};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW,
};
use windows::core::w;

use crate::clock::QpcMap;
use crate::com::ComGuard;

const RATE: u32 = 48_000;
const CHANNELS: usize = 2;
const BYTES_PER_FRAME: usize = CHANNELS * size_of::<f32>();
const TICK_MS: u32 = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmBlock {
    pub first_frame: u64,
    pub samples: Vec<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTarget {
    Process(u32),
    System,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("wasapi: {0}")]
    Wasapi(#[from] WasapiError),
    #[error("windows: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("audio thread ended before it was ready")]
    ThreadGone,
    #[error("could not spawn audio thread: {0}")]
    Spawn(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct AudioCapture {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AudioCapture {
    pub fn start(
        target: AudioTarget,
        qpc: QpcMap,
        t0: Hns,
        out: Sender<PcmBlock>,
    ) -> Result<Self, AudioError> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("greyscreen-audio".into())
            .spawn(move || run(target, qpc, t0, out, thread_stop, ready_tx))?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                stop,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                join(thread);
                Err(error)
            }
            Err(_) => {
                join(thread);
                Err(AudioError::ThreadGone)
            }
        }
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            join(thread);
        }
    }
}

fn join(thread: JoinHandle<()>) {
    if thread.join().is_err() {
        tracing::warn!("audio thread panicked");
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct Mmcss(Option<HANDLE>);

impl Mmcss {
    fn enter() -> Self {
        let mut task = 0u32;
        // SAFETY: the task index pointer refers to a live local u32 and the name is a static wide string.
        match unsafe { AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut task) } {
            Ok(handle) => Self(Some(handle)),
            Err(error) => {
                tracing::warn!(%error, "MMCSS registration failed");
                Self(None)
            }
        }
    }
}

impl Drop for Mmcss {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            // SAFETY: the handle came from a successful AvSetMmThreadCharacteristicsW on this thread.
            if let Err(error) = unsafe { AvRevertMmThreadCharacteristics(handle) } {
                tracing::warn!(%error, "MMCSS revert failed");
            }
        }
    }
}

struct Stream {
    client: AudioClient,
    capture: AudioCaptureClient,
    event: wasapi::Handle,
}

fn open(target: AudioTarget) -> Result<Stream, AudioError> {
    let mut client = match target {
        AudioTarget::Process(pid) => AudioClient::new_application_loopback_client(pid, true)?,
        AudioTarget::System => DeviceEnumerator::new()?
            .get_default_device(Direction::Render)?
            .get_iaudioclient()?,
    };
    let format = WaveFormat::new(32, 32, SampleType::Float, RATE as usize, CHANNELS, None);
    client.initialize_client(
        &format,
        Direction::Capture,
        StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: 0,
        },
        None,
    )?;
    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    client.start_stream()?;
    Ok(Stream {
        client,
        capture,
        event,
    })
}

fn notify(ready: &Sender<Result<(), AudioError>>, result: Result<(), AudioError>) {
    if ready.send(result).is_err() {
        tracing::debug!("audio start result had no listener");
    }
}

fn run(
    target: AudioTarget,
    qpc: QpcMap,
    t0: Hns,
    out: Sender<PcmBlock>,
    stop: Arc<AtomicBool>,
    ready: Sender<Result<(), AudioError>>,
) {
    let _com = match ComGuard::mta() {
        Ok(guard) => guard,
        Err(error) => {
            notify(&ready, Err(error.into()));
            return;
        }
    };
    let _mmcss = Mmcss::enter();
    let stream = match open(target) {
        Ok(stream) => stream,
        Err(error) => {
            notify(&ready, Err(error));
            return;
        }
    };
    notify(&ready, Ok(()));
    capture_loop(&stream, qpc, t0, &out, &stop);
    if let Err(error) = stream.client.stop_stream() {
        tracing::warn!(%error, "stopping audio stream failed");
    }
    if let Err(error) = stream.client.reset_stream() {
        tracing::warn!(%error, "resetting audio stream failed");
    }
}

fn capture_loop(stream: &Stream, qpc: QpcMap, t0: Hns, out: &Sender<PcmBlock>, stop: &AtomicBool) {
    let mut timeline = AudioTimeline::new(t0, RATE);
    let mut buffer = Vec::new();
    while !stop.load(Ordering::Acquire) {
        match stream.event.wait_for_event(TICK_MS) {
            Ok(()) => {}
            Err(WasapiError::EventTimeout) => {
                if let Some(Span::Silence(frames)) = timeline.on_timeout(mono_now()) {
                    let block = PcmBlock {
                        first_frame: timeline.frames_out() - frames as u64,
                        samples: vec![0; frames * CHANNELS],
                    };
                    if out.send(block).is_err() {
                        return;
                    }
                }
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, "audio wait failed");
                return;
            }
        }
        match drain(stream, qpc, &mut timeline, &mut buffer, out) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!(%error, "audio read failed");
                return;
            }
        }
    }
}

fn drain(
    stream: &Stream,
    qpc: QpcMap,
    timeline: &mut AudioTimeline,
    buffer: &mut Vec<u8>,
    out: &Sender<PcmBlock>,
) -> Result<bool, WasapiError> {
    while let Some(frames) = stream.capture.get_next_packet_size()? {
        if frames == 0 {
            break;
        }
        buffer.clear();
        buffer.resize(frames as usize * BYTES_PER_FRAME, 0);
        let (read, info) = stream.capture.read_from_device(buffer)?;
        let now = mono_now();
        if read == 0 {
            break;
        }
        if info.flags.data_discontinuity {
            tracing::debug!("audio data discontinuity");
        }
        let ts = (!info.flags.timestamp_error && info.timestamp != 0)
            .then(|| qpc.to_mono(info.timestamp as Hns));
        let first_frame = timeline.frames_out();
        let spans = timeline.on_packet_now(ts, now, read as usize, info.flags.silent);
        let samples = render(&spans, &buffer[..read as usize * BYTES_PER_FRAME]);
        if samples.is_empty() {
            continue;
        }
        let block = PcmBlock {
            first_frame,
            samples,
        };
        if out.send(block).is_err() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn render(spans: &[Span], bytes: &[u8]) -> Vec<i16> {
    let mut samples = Vec::new();
    for span in spans {
        match *span {
            Span::Silence(n) => samples.resize(samples.len() + n * CHANNELS, 0),
            Span::Data { skip, take } => {
                let start = skip * BYTES_PER_FRAME;
                let end = start + take * BYTES_PER_FRAME;
                samples.extend(
                    bytes[start..end]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|chunk| f32_to_i16(f32::from_le_bytes(*chunk))),
                );
            }
        }
    }
    samples
}

fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}
