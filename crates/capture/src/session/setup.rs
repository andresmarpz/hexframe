use crossbeam_channel::Sender;
use greyscreen_core::frame_timing::{Pacer, Watchdog};
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent};
use greyscreen_core::time::mono_now;
use greyscreen_mp4::avc::{NAL_PPS, NAL_SPS, nal_type, split_annexb};

use super::Flags;
use super::audio::AudioPipe;
use super::emit::{Sink, StartInfo};
use super::facts;
use super::pick::{self, Stage};
use super::video::Video;
use crate::clock::QpcMap;
use crate::com::{ComGuard, MfGuard};
use crate::d3d::luid_u64;
use crate::pacer::Stats;
use crate::power::SystemPowerGuard;
use crate::procstats::ProcessSampler;

const BORDER_SHOWN: &str = "Windows shows a yellow border around the captured window while Greyscreen records. It is not in the video.";

pub(crate) fn build(
    target: &CaptureTarget,
    cfg: &CaptureConfig,
    out: Sender<MediaEvent>,
    flags: Flags,
) -> Result<Video, String> {
    tracing::info!(step = "com", "capture setup");
    let com = ComGuard::mta().map_err(|e| format!("COM init failed: {e}"))?;
    tracing::info!(step = "media foundation", "capture setup");
    let mf = MfGuard::start().map_err(|e| format!("Media Foundation start failed: {e}"))?;
    tracing::info!(step = "gpu", "capture setup");
    let qpc = QpcMap::now();
    let Stage {
        gpu,
        wgc,
        frame,
        client_area,
        crop,
        size,
        converter,
        encoder,
        cfg,
        video_cfg,
    } = pick::first_working(target, cfg, qpc, &flags)?;
    log_capture_path(frame, client_area.client_size());
    tracing::info!(encoder = %encoder.encoder_name(), "video encoder chosen");
    let mut warnings = Vec::new();
    if !wgc.border_hidden() {
        warnings.push(BORDER_SHOWN.to_owned());
    }
    tracing::info!(step = "audio", enabled = cfg.audio, "capture setup");
    let audio = if cfg.audio {
        match AudioPipe::start(target, qpc, cfg.audio_bitrate) {
            Ok(audio) => Some(audio),
            Err(message) => {
                warnings.push(format!("recording without audio: {message}"));
                None
            }
        }
    } else {
        None
    };
    let adapter_luid = luid_u64(gpu.adapter_luid);
    let t0 = mono_now();
    if let Some(audio) = &audio {
        audio.begin(t0);
    }
    let mut sink = Sink::new(
        out,
        StartInfo {
            t0,
            width: size.0,
            height: size.1,
            fps: cfg.fps,
            audio: audio.as_ref().map(|a| a.format.clone()),
            encoder: encoder.encoder_name(),
        },
    );
    for message in warnings {
        sink.warning(message);
    }
    if cfg.diagnostics {
        sink.setup(facts::describe(facts::Inputs {
            target,
            cfg: &video_cfg,
            gpu: &gpu,
            frame,
            crop,
            window: client_area.client_size(),
            encode: size,
            encoder: encoder.encoder_name(),
        }));
    }
    if let Some((sps, pps)) = parameter_sets(encoder.sequence_header()) {
        sink.set_fallback(sps, pps);
    }
    let mut power = SystemPowerGuard::system();
    power.hold();
    Ok(Video {
        wgc,
        converter,
        client_area,
        encoder,
        audio,
        sink,
        pacer: Pacer::new(t0, cfg.fps),
        watchdog: Watchdog::new(t0),
        flags,
        gpu,
        mf,
        com,
        scratch: Vec::new(),
        stats: Stats::new(),
        sampler: cfg.diagnostics.then(|| {
            ProcessSampler::new(
                match target {
                    CaptureTarget::GameWindow { hwnd, .. } => Some(*hwnd),
                    CaptureTarget::PrimaryMonitor => None,
                },
                Some(adapter_luid),
            )
        }),
        seq: 0,
        last_seq: 0,
        soft_errors: 0,
        power,
    })
}

fn log_capture_path(frame: (u32, u32), client: Option<(u32, u32)>) {
    match client {
        Some(client) if client != frame => tracing::info!(
            frame_width = frame.0,
            frame_height = frame.1,
            client_width = client.0,
            client_height = client.1,
            "capture size differs from the window client size: Windows scales or composes the window"
        ),
        Some(_) => tracing::info!(
            width = frame.0,
            height = frame.1,
            "capture size matches the window client size"
        ),
        None => {}
    }
}

fn parameter_sets(header: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut sps = None;
    let mut pps = None;
    for nal in split_annexb(header) {
        match nal_type(nal[0]) {
            NAL_SPS => sps = Some(nal.to_vec()),
            NAL_PPS => pps = Some(nal.to_vec()),
            _ => {}
        }
    }
    sps.zip(pps)
}
