use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use greyscreen_core::health::CaptureStats;
use greyscreen_core::media::{AudioFormat, CaptureTarget, Packet, Track};
use greyscreen_core::time::Hns;

use super::emit::Sink;
use crate::aac::AacEncoder;
use crate::audio::{AudioCapture, AudioCounters, AudioTarget, PcmBlock};
use crate::clock::QpcMap;
use crate::com::{ComGuard, MfGuard};

const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: u16 = 2;
const AAC_FRAME_TICKS: i64 = 1024;
const PCM_QUEUE: usize = 256;

type Ready = Sender<Result<Vec<u8>, String>>;

enum AudioMsg {
    Packet(Packet),
    Warning(String),
}

pub(crate) struct AudioPipe {
    capture: Option<AudioCapture>,
    worker: JoinHandle<()>,
    rx: Receiver<AudioMsg>,
    t0_tx: Sender<Hns>,
    counters: Arc<AudioCounters>,
    pcm_probe: Receiver<PcmBlock>,
    pub(crate) format: AudioFormat,
}

impl AudioPipe {
    pub(crate) fn start(target: &CaptureTarget, qpc: QpcMap, bitrate: u32) -> Result<Self, String> {
        let (pcm_tx, pcm_rx) = crossbeam_channel::bounded(PCM_QUEUE);
        let pcm_probe = pcm_rx.clone();
        let (msg_tx, msg_rx) = crossbeam_channel::unbounded();
        let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
        let worker = std::thread::Builder::new()
            .name("greyscreen-aac".into())
            .spawn(move || aac_main(bitrate, pcm_rx, msg_tx, ready_tx))
            .map_err(|e| format!("could not spawn the AAC thread: {e}"))?;
        let asc = match ready_rx.recv() {
            Ok(Ok(asc)) => asc,
            Ok(Err(message)) => {
                join(worker);
                return Err(message);
            }
            Err(_) => {
                join(worker);
                return Err("the AAC thread ended before it was ready".into());
            }
        };
        let audio_target = match *target {
            CaptureTarget::GameWindow { pid, .. } => AudioTarget::Process(pid),
            CaptureTarget::PrimaryMonitor => AudioTarget::System,
        };
        let (t0_tx, t0_rx) = crossbeam_channel::bounded(1);
        let counters = Arc::new(AudioCounters::default());
        let capture = match AudioCapture::start(audio_target, qpc, t0_rx, pcm_tx, counters.clone())
        {
            Ok(capture) => capture,
            Err(error) => {
                join(worker);
                return Err(format!("audio capture failed: {error}"));
            }
        };
        Ok(Self {
            capture: Some(capture),
            worker,
            rx: msg_rx,
            t0_tx,
            counters,
            pcm_probe,
            format: AudioFormat {
                sample_rate: SAMPLE_RATE,
                channels: CHANNELS,
                bitrate,
                asc,
            },
        })
    }

    pub(crate) fn fill_stats(&self, stats: &mut CaptureStats) {
        self.counters.fill(stats);
        stats.pcm_queue = u32::try_from(self.pcm_probe.len()).unwrap_or(u32::MAX);
    }

    pub(crate) fn begin(&self, t0: Hns) {
        self.t0_tx.send(t0).ok();
    }

    pub(crate) fn drain(&self, sink: &mut Sink) -> u64 {
        let mut packets = 0;
        while let Ok(message) = self.rx.try_recv() {
            packets += forward(message, sink);
        }
        packets
    }

    pub(crate) fn finish(mut self, sink: &mut Sink) -> u64 {
        if let Some(capture) = self.capture.take() {
            capture.stop();
        }
        let mut packets = 0;
        for message in self.rx.iter() {
            packets += forward(message, sink);
        }
        join(self.worker);
        packets
    }
}

fn forward(message: AudioMsg, sink: &mut Sink) -> u64 {
    match message {
        AudioMsg::Packet(packet) => {
            sink.packet(packet);
            1
        }
        AudioMsg::Warning(text) => {
            sink.warning(text);
            0
        }
    }
}

fn join(thread: JoinHandle<()>) {
    if thread.join().is_err() {
        tracing::warn!("AAC thread panicked");
    }
}

fn aac_main(bitrate: u32, pcm: Receiver<PcmBlock>, out: Sender<AudioMsg>, ready: Ready) {
    let _com = match ComGuard::mta() {
        Ok(guard) => guard,
        Err(e) => return report(&ready, Err(format!("COM init failed: {e}"))),
    };
    let _mf = match MfGuard::start() {
        Ok(guard) => guard,
        Err(e) => return report(&ready, Err(format!("Media Foundation start failed: {e}"))),
    };
    let mut encoder = match AacEncoder::new(SAMPLE_RATE, CHANNELS, bitrate) {
        Ok(encoder) => encoder,
        Err(e) => return report(&ready, Err(format!("AAC encoder failed: {e}"))),
    };
    report(&ready, Ok(encoder.asc()));
    let mut next = 0i64;
    let mut buf = Vec::new();
    for block in pcm.iter() {
        let result = encoder.encode(&block, &mut buf);
        emit(&out, &mut buf, &mut next);
        if let Err(e) = result {
            out.send(AudioMsg::Warning(format!("audio encoding stopped: {e}")))
                .ok();
            return;
        }
    }
    if let Err(e) = encoder.finish(&mut buf) {
        out.send(AudioMsg::Warning(format!("audio flush failed: {e}")))
            .ok();
    }
    emit(&out, &mut buf, &mut next);
}

fn report(ready: &Ready, result: Result<Vec<u8>, String>) {
    ready.send(result).ok();
}

fn emit(out: &Sender<AudioMsg>, frames: &mut Vec<Vec<u8>>, next: &mut i64) {
    for frame in frames.drain(..) {
        let ticks = *next * AAC_FRAME_TICKS;
        *next += 1;
        out.send(AudioMsg::Packet(Packet {
            track: Track::Audio,
            dts: ticks,
            pts: ticks,
            duration: AAC_FRAME_TICKS as u32,
            key: true,
            data: Arc::from(frame),
        }))
        .ok();
    }
}
