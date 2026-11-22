use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, SendTimeoutError, Sender};
use greyscreen_core::health::CaptureStats;
use greyscreen_core::media::{AudioTimeline, Span};
use greyscreen_core::time::{Hns, mono_now};
use wasapi::WasapiError;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW,
};
use windows::core::w;

use crate::clock::QpcMap;
use crate::com::ComGuard;

mod source;

pub use source::{CaptureClient, OpenSource, PacketInfo, SourceError, open_device};

pub(crate) const RATE: u32 = 48_000;
pub(crate) const CHANNELS: usize = 2;
const BYTES_PER_FRAME: usize = CHANNELS * size_of::<f32>();
const TICK_MS: u32 = 50;

#[derive(Debug, Default)]
pub struct AudioCounters {
    discontinuities: AtomicU64,
    gaps: AtomicU64,
    gap_longest_ms: AtomicU32,
    gap_total_ms: AtomicU64,
    idle_ms: AtomicU64,
    timestamp_gaps_over_5s: AtomicU32,
    largest_timestamp_gap_ms: AtomicU64,
    queue_depth: AtomicU32,
    failure: Mutex<Option<String>>,
}

impl AudioCounters {
    fn gap(&self, frames: usize) {
        let ms = frames_to_ms(frames);
        self.gaps.fetch_add(1, Ordering::Relaxed);
        self.gap_total_ms
            .fetch_add(u64::from(ms), Ordering::Relaxed);
        self.gap_longest_ms.fetch_max(ms, Ordering::Relaxed);
    }

    fn idle(&self, frames: usize) {
        self.idle_ms
            .fetch_add(u64::from(frames_to_ms(frames)), Ordering::Relaxed);
    }

    fn timestamp_gaps(&self, timeline: &AudioTimeline) {
        self.timestamp_gaps_over_5s
            .store(timeline.big_gaps(), Ordering::Relaxed);
        let ms = u64::try_from(timeline.largest_gap() / 10_000).unwrap_or(0);
        self.largest_timestamp_gap_ms.store(ms, Ordering::Relaxed);
    }

    fn fail(&self, text: String) {
        if let Ok(mut slot) = self.failure.lock() {
            slot.get_or_insert(text);
        }
    }

    pub fn take_failure(&self) -> Option<String> {
        self.failure.lock().ok().and_then(|mut slot| slot.take())
    }

    pub fn fill(&self, stats: &mut CaptureStats) {
        stats.audio_discontinuities = self.discontinuities.load(Ordering::Relaxed);
        stats.audio_gaps = self.gaps.load(Ordering::Relaxed);
        stats.audio_gap_longest_ms = self.gap_longest_ms.load(Ordering::Relaxed);
        stats.audio_gap_total_ms = self.gap_total_ms.load(Ordering::Relaxed);
        stats.audio_idle_ms = self.idle_ms.load(Ordering::Relaxed);
        stats.audio_timestamp_gaps_over_5s = self.timestamp_gaps_over_5s.load(Ordering::Relaxed);
        stats.audio_largest_timestamp_gap_ms =
            self.largest_timestamp_gap_ms.load(Ordering::Relaxed);
        stats.pcm_queue = self.queue_depth.load(Ordering::Relaxed);
    }
}

const SEND_SLICE: Duration = Duration::from_millis(20);

fn send_block(
    out: &Sender<PcmBlock>,
    block: PcmBlock,
    stop: &AtomicBool,
    counters: &AudioCounters,
) -> bool {
    let mut pending = block;
    loop {
        match out.send_timeout(pending, SEND_SLICE) {
            Ok(()) => {
                let depth = u32::try_from(out.len()).unwrap_or(u32::MAX);
                counters.queue_depth.store(depth, Ordering::Relaxed);
                return true;
            }
            Err(SendTimeoutError::Disconnected(_)) => return false,
            Err(SendTimeoutError::Timeout(block)) => {
                if stop.load(Ordering::Acquire) {
                    return false;
                }
                pending = block;
            }
        }
    }
}

fn frames_to_ms(frames: usize) -> u32 {
    u32::try_from(frames as u64 * 1000 / u64::from(RATE)).unwrap_or(u32::MAX)
}

fn gap_frames(spans: &[Span], silent: bool) -> usize {
    let gap_spans = if silent {
        spans.len().saturating_sub(1)
    } else {
        spans.len()
    };
    spans[..gap_spans]
        .iter()
        .map(|span| match *span {
            Span::Silence(frames) => frames,
            Span::Data { .. } => 0,
        })
        .sum()
}

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
        t0: Receiver<Hns>,
        out: Sender<PcmBlock>,
        counters: Arc<AudioCounters>,
    ) -> Result<Self, AudioError> {
        Self::start_with(open_device(target), qpc, t0, out, counters)
    }

    pub fn start_with(
        open: OpenSource,
        qpc: QpcMap,
        t0: Receiver<Hns>,
        out: Sender<PcmBlock>,
        counters: Arc<AudioCounters>,
    ) -> Result<Self, AudioError> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("greyscreen-audio".into())
            .spawn(move || run(open, qpc, t0, out, counters, thread_stop, ready_tx))?;
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
        match fault!(Mmcss)
            // SAFETY: the task index pointer refers to a live local u32 and the name is a static wide string.
            .and_then(|()| unsafe { AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut task) })
        {
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

fn notify(ready: &Sender<Result<(), AudioError>>, result: Result<(), AudioError>) {
    if ready.send(result).is_err() {
        tracing::debug!("audio start result had no listener");
    }
}

fn run(
    open: OpenSource,
    qpc: QpcMap,
    t0: Receiver<Hns>,
    out: Sender<PcmBlock>,
    counters: Arc<AudioCounters>,
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
    let mut stream = match open() {
        Ok(stream) => stream,
        Err(error) => {
            notify(&ready, Err(error));
            return;
        }
    };
    notify(&ready, Ok(()));
    if let Some(t0) = wait_for_t0(&t0, &stop) {
        capture_loop(stream.as_mut(), qpc, t0, &out, &counters, &stop);
        tracing::info!(
            largest_timestamp_gap_ms = counters.largest_timestamp_gap_ms.load(Ordering::Relaxed),
            timestamp_gaps_over_5s = counters.timestamp_gaps_over_5s.load(Ordering::Relaxed),
            "audio timestamp gaps"
        );
    }
    stream.stop();
}

fn wait_for_t0(t0: &Receiver<Hns>, stop: &AtomicBool) -> Option<Hns> {
    loop {
        match t0.recv_timeout(Duration::from_millis(50)) {
            Ok(value) => return Some(value),
            Err(RecvTimeoutError::Timeout) if !stop.load(Ordering::Acquire) => {}
            Err(_) => return None,
        }
    }
}

fn capture_loop(
    stream: &mut dyn CaptureClient,
    qpc: QpcMap,
    t0: Hns,
    out: &Sender<PcmBlock>,
    counters: &AudioCounters,
    stop: &AtomicBool,
) {
    let Some(mut timeline) = AudioTimeline::new(t0, RATE) else {
        return;
    };
    let mut buffer = Vec::new();
    while !stop.load(Ordering::Acquire) {
        match stream.wait(TICK_MS) {
            Ok(true) => {}
            Ok(false) => {
                if let Some(Span::Silence(frames)) = timeline.on_timeout(mono_now()) {
                    counters.idle(frames);
                    let chunk = (RATE as usize).max(1);
                    let mut first_frame = timeline.frames_out() - frames as u64;
                    let mut left = frames;
                    while left > 0 {
                        let n = left.min(chunk);
                        let block = PcmBlock {
                            first_frame,
                            samples: vec![0; n * CHANNELS],
                        };
                        if !send_block(out, block, stop, counters) {
                            return;
                        }
                        first_frame += n as u64;
                        left -= n;
                    }
                }
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, "audio wait failed");
                report_loss(&error, counters, stop);
                return;
            }
        }
        match drain(
            &mut *stream,
            qpc,
            &mut timeline,
            &mut buffer,
            out,
            counters,
            stop,
        ) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!(%error, "audio read failed");
                report_loss(&error, counters, stop);
                return;
            }
        }
    }
}

fn report_loss(error: &SourceError, counters: &AudioCounters, stop: &AtomicBool) {
    if stop.load(Ordering::Acquire) {
        return;
    }
    let text = match error {
        SourceError::DeviceInvalidated => "audio device was lost, audio stopped".to_string(),
        SourceError::Failed(reason) => format!("audio capture stopped: {reason}"),
    };
    counters.fail(text);
}

fn drain(
    stream: &mut dyn CaptureClient,
    qpc: QpcMap,
    timeline: &mut AudioTimeline,
    buffer: &mut Vec<u8>,
    out: &Sender<PcmBlock>,
    counters: &AudioCounters,
    stop: &AtomicBool,
) -> Result<bool, SourceError> {
    while let Some(frames) = stream.next_packet_size()? {
        if frames == 0 {
            break;
        }
        buffer.clear();
        buffer.resize(frames as usize * BYTES_PER_FRAME, 0);
        let (read, info) = stream.read(buffer)?;
        let now = mono_now();
        if read == 0 {
            break;
        }
        if info.discontinuity {
            counters.discontinuities.fetch_add(1, Ordering::Relaxed);
            tracing::debug!("audio data discontinuity");
        }
        let ts = (!info.timestamp_error && info.timestamp != 0)
            .then(|| qpc.to_mono(info.timestamp as Hns));
        let first_frame = timeline.frames_out();
        let first_packet = first_frame == 0;
        let spans = timeline.on_packet_now(ts, now, read as usize, info.silent);
        counters.timestamp_gaps(timeline);
        let missing = gap_frames(&spans, info.silent);
        if first_packet {
            counters.idle(missing);
        } else if missing > 0 {
            counters.gap(missing);
        }
        let mut start = first_frame;
        for span in &spans {
            let samples = render(span, &buffer[..read as usize * BYTES_PER_FRAME]);
            if samples.is_empty() {
                continue;
            }
            let frames = (samples.len() / CHANNELS) as u64;
            let block = PcmBlock {
                first_frame: start,
                samples,
            };
            if !send_block(out, block, stop, counters) {
                return Ok(false);
            }
            start += frames;
        }
    }
    Ok(true)
}

fn render(span: &Span, bytes: &[u8]) -> Vec<i16> {
    match *span {
        Span::Silence(n) => vec![0; n * CHANNELS],
        Span::Data { skip, take } => {
            let start = skip * BYTES_PER_FRAME;
            let end = start + take * BYTES_PER_FRAME;
            bytes[start..end]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|chunk| f32_to_i16(f32::from_le_bytes(*chunk)))
                .collect()
        }
    }
}

fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}

#[cfg(test)]
mod tests;
