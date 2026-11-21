use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent, encode_size};
use greyscreen_core::time::mono_now;
use greyscreen_mp4::avc::{NAL_PPS, NAL_SPS, nal_type, split_annexb};

use super::Flags;
use super::audio::AudioPipe;
use super::emit::{Sink, StartInfo};
use super::video::Video;
use crate::client_area::ClientArea;
use crate::clock::QpcMap;
use crate::com::{ComGuard, MfGuard};
use crate::d3d::GpuDevice;
use crate::h264::H264Encoder;
use crate::pacer::{Pacer, Stats};
use crate::vproc::VideoConverter;
use crate::watchdog::Watchdog;
use crate::wgc::{WgcCapture, WgcOptions};

const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(2);
const FIRST_FRAME_POLL: Duration = Duration::from_millis(10);
const NO_FRAMES: &str = "no frames from the game window: if League runs in exclusive fullscreen, switch it to Borderless";

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
    let gpu = match *target {
        CaptureTarget::GameWindow { hwnd, .. } => GpuDevice::for_window(hwnd),
        CaptureTarget::PrimaryMonitor => GpuDevice::for_primary_monitor(),
    }
    .map_err(|e| format!("no usable GPU: {e}"))?;
    let qpc = QpcMap::now();
    tracing::info!(step = "graphics capture", adapter = %gpu.adapter_name, "capture setup");
    let wgc = WgcCapture::start(
        &gpu,
        target,
        qpc,
        WgcOptions {
            fps: cfg.fps,
            cursor: cfg.cursor,
        },
    )
    .map_err(|e| format!("screen capture failed to start: {e}"))?;
    tracing::info!(step = "first frame", "capture setup");
    let frame = first_frame_size(&wgc, &flags)?;
    let mut client_area = ClientArea::for_target(target);
    let native = client_area.crop(frame).size();
    let size = encode_size(native, cfg.max_height);
    tracing::info!(
        step = "video converter",
        width = size.0,
        height = size.1,
        "capture setup"
    );
    let converter = VideoConverter::new(&gpu, size.0, size.1, cfg.fps)
        .map_err(|e| format!("video converter failed: {e}"))?;
    let video_cfg = cfg.for_encode_size(size.0, size.1);
    tracing::info!(
        width = size.0,
        height = size.1,
        bitrate = video_cfg.video_bitrate,
        peak = video_cfg.video_peak,
        "encode size"
    );
    tracing::info!(step = "encoder", "capture setup");
    let encoder = H264Encoder::new(&gpu, size.0, size.1, cfg.fps, &video_cfg)
        .map_err(|e| format!("video encoder failed: {e}"))?
        .with_releaser(converter.releaser());
    let mut warning = None;
    tracing::info!(step = "audio", enabled = cfg.audio, "capture setup");
    let audio = if cfg.audio {
        match AudioPipe::start(target, qpc, cfg.audio_bitrate) {
            Ok(audio) => Some(audio),
            Err(message) => {
                warning = Some(format!("recording without audio: {message}"));
                None
            }
        }
    } else {
        None
    };
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
    if let Some(message) = warning {
        sink.warning(message);
    }
    if let Some((sps, pps)) = parameter_sets(encoder.sequence_header()) {
        sink.set_fallback(sps, pps);
    }
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
        seq: 0,
        soft_errors: 0,
    })
}

fn first_frame_size(wgc: &WgcCapture, flags: &Flags) -> Result<(u32, u32), String> {
    let deadline = Instant::now() + FIRST_FRAME_TIMEOUT;
    loop {
        if let Some((_, size, _)) = wgc.latest() {
            return Ok(size);
        }
        if wgc.is_closed() {
            return Err("the captured window closed during startup".into());
        }
        if flags.stop.load(Ordering::Acquire) {
            return Err("capture stopped during startup".into());
        }
        if Instant::now() >= deadline {
            return Err(NO_FRAMES.into());
        }
        std::thread::sleep(FIRST_FRAME_POLL);
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
