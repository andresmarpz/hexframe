use std::time::{Duration, Instant};

use greyscreen_core::crop::CropRect;
use greyscreen_core::encoder::{self, Candidate, EncoderKind, OpenFailure};
use greyscreen_core::media::{CaptureConfig, CaptureTarget, encode_size};
use greyscreen_core::problem::NO_FRAMES;

use super::{Flags, STOPPED_DURING_SETUP, facts};
use crate::adapters;
use crate::client_area::ClientArea;
use crate::clock::QpcMap;
use crate::d3d::GpuDevice;
use crate::h264::H264Encoder;
use crate::vproc::VideoConverter;
use crate::wgc::{WgcCapture, WgcOptions};

const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(2);
const FIRST_FRAME_POLL: Duration = Duration::from_millis(10);

pub(super) struct Stage {
    pub(super) gpu: GpuDevice,
    pub(super) wgc: WgcCapture,
    pub(super) frame: (u32, u32),
    pub(super) client_area: ClientArea,
    pub(super) crop: CropRect,
    pub(super) size: (u32, u32),
    pub(super) converter: VideoConverter,
    pub(super) encoder: H264Encoder,
    pub(super) cfg: CaptureConfig,
    pub(super) video_cfg: CaptureConfig,
    pub(super) fallback_warning: Option<String>,
}

enum Failure {
    Setup(String),
    Encoders(Vec<String>),
}

pub(super) fn first_working(
    target: &CaptureTarget,
    cfg: &CaptureConfig,
    qpc: QpcMap,
    flags: &Flags,
) -> Result<Stage, String> {
    let groups = adapter_groups(&encoder::plan(&adapters::for_target(target)));
    let mut setup_error = None;
    let mut encoder_errors = Vec::new();
    let mut hardware_failures = Vec::new();
    for (adapter, kinds) in groups {
        match build_on(
            adapter,
            &kinds,
            target,
            cfg,
            qpc,
            flags,
            &mut hardware_failures,
        ) {
            Ok(mut stage) => {
                stage.fallback_warning =
                    encoder::fallback_warning(stage.encoder.kind(), &hardware_failures);
                return Ok(stage);
            }
            Err(Failure::Setup(message)) => {
                tracing::warn!(%message, "capture setup failed on this adapter");
                setup_error.get_or_insert(message);
            }
            Err(Failure::Encoders(errors)) => encoder_errors.extend(errors),
        }
        if flags.stopped() {
            break;
        }
    }
    if encoder_errors.is_empty() {
        Err(setup_error.unwrap_or_else(|| "no usable GPU".to_owned()))
    } else {
        Err(format!(
            "video encoder failed: {}",
            encoder_errors.join("; ")
        ))
    }
}

fn check_stop(flags: &Flags) -> Result<(), Failure> {
    if flags.stopped() {
        Err(Failure::Setup(STOPPED_DURING_SETUP.into()))
    } else {
        Ok(())
    }
}

fn adapter_groups(plan: &[Candidate]) -> Vec<(Option<u64>, Vec<EncoderKind>)> {
    let mut groups: Vec<(Option<u64>, Vec<EncoderKind>)> = Vec::new();
    for candidate in plan {
        match groups
            .iter_mut()
            .find(|(a, _)| *a == Some(candidate.adapter))
        {
            Some((_, kinds)) => kinds.push(candidate.kind),
            None => groups.push((Some(candidate.adapter), vec![candidate.kind])),
        }
    }
    if groups.is_empty() {
        groups.push((None, vec![EncoderKind::Hardware, EncoderKind::Software]));
    }
    groups
}

fn build_on(
    adapter: Option<u64>,
    kinds: &[EncoderKind],
    target: &CaptureTarget,
    cfg: &CaptureConfig,
    qpc: QpcMap,
    flags: &Flags,
    hardware_failures: &mut Vec<(OpenFailure, u32)>,
) -> Result<Stage, Failure> {
    check_stop(flags)?;
    let gpu = match adapter {
        Some(luid) => GpuDevice::for_luid(luid),
        None => GpuDevice::for_target(target),
    }
    .map_err(|e| Failure::Setup(format!("no usable GPU: {e}")))?;
    check_stop(flags)?;
    tracing::info!(step = "graphics capture", adapter = %gpu.adapter_name, "capture setup");
    let wgc = WgcCapture::start(
        &gpu,
        target,
        qpc,
        WgcOptions {
            fps: cfg.fps,
            cursor: cfg.cursor,
            display_hz: facts::display_hz(target),
        },
    )
    .map_err(|e| Failure::Setup(format!("screen capture failed to start: {e}")))?;
    if let Err(failure) = check_stop(flags) {
        wgc.stop();
        return Err(failure);
    }
    tracing::info!(step = "first frame", "capture setup");
    let frame = match first_frame_size(&wgc, flags) {
        Ok(frame) => frame,
        Err(message) => {
            wgc.stop();
            return Err(Failure::Setup(message));
        }
    };
    let mut client_area = ClientArea::for_target(target);
    let crop = client_area.crop(frame);
    let mut errors = Vec::new();
    for &kind in kinds {
        if let Err(failure) = check_stop(flags) {
            wgc.stop();
            return Err(failure);
        }
        let kind_cfg = match kind {
            EncoderKind::Hardware => cfg.clone(),
            EncoderKind::Software => encoder::software_config(cfg),
        };
        match open_encoder(&gpu, kind, &kind_cfg, crop.size()) {
            Ok((size, converter, encoder, video_cfg)) => {
                if let Err(failure) = check_stop(flags) {
                    wgc.stop();
                    return Err(failure);
                }
                return Ok(Stage {
                    gpu,
                    wgc,
                    frame,
                    client_area,
                    crop,
                    size,
                    converter,
                    encoder,
                    cfg: kind_cfg,
                    video_cfg,
                    fallback_warning: None,
                });
            }
            Err(error) => {
                let label = encoder::label(kind, gpu.vendor_id, &gpu.adapter_name);
                tracing::warn!(encoder = %label, %error, "encoder failed, trying the next one");
                if kind == EncoderKind::Hardware {
                    hardware_failures.push((OpenFailure::from_text(&error), gpu.vendor_id));
                }
                errors.push(format!("{label} on {}: {error}", gpu.adapter_name));
            }
        }
    }
    wgc.stop();
    Err(Failure::Encoders(errors))
}

fn open_encoder(
    gpu: &GpuDevice,
    kind: EncoderKind,
    cfg: &CaptureConfig,
    native: (u32, u32),
) -> Result<((u32, u32), VideoConverter, H264Encoder, CaptureConfig), String> {
    fault!(EncoderCandidate).map_err(|error| error.to_string())?;
    let size = encode_size(native, cfg.max_height);
    tracing::info!(
        step = "video converter",
        width = size.0,
        height = size.1,
        "capture setup"
    );
    let converter = VideoConverter::new(gpu, size.0, size.1, cfg.fps)
        .map_err(|e| format!("video converter failed: {e}"))?;
    let video_cfg = cfg.for_encode_size(size.0, size.1);
    tracing::info!(
        step = "encoder",
        ?kind,
        width = size.0,
        height = size.1,
        bitrate = video_cfg.video_bitrate,
        peak = video_cfg.video_peak,
        "capture setup"
    );
    let encoder = H264Encoder::new(gpu, kind, size.0, size.1, cfg.fps, &video_cfg)
        .map_err(|e| e.to_string())?
        .with_releaser(converter.releaser());
    Ok((size, converter, encoder, video_cfg))
}

fn first_frame_size(wgc: &WgcCapture, flags: &Flags) -> Result<(u32, u32), String> {
    fault!(FirstFrameWait).map_err(|error| error.to_string())?;
    let deadline = Instant::now() + FIRST_FRAME_TIMEOUT;
    loop {
        if let Some((_, size, _)) = wgc.latest() {
            return Ok(size);
        }
        if wgc.is_closed() {
            return Err("the captured window closed during startup".into());
        }
        if flags.stopped() {
            return Err(STOPPED_DURING_SETUP.into());
        }
        if Instant::now() >= deadline {
            return Err(NO_FRAMES.into());
        }
        std::thread::sleep(FIRST_FRAME_POLL);
    }
}
