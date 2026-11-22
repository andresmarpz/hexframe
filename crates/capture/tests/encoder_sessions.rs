#![cfg(windows)]

use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use greyscreen_capture::WindowsMediaSource;
use greyscreen_capture::adapters;
use greyscreen_capture::com::{ComGuard, MfGuard};
use greyscreen_capture::d3d::GpuDevice;
use greyscreen_capture::h264::H264Encoder;
use greyscreen_core::encoder::{BUSY_HELP, EncoderKind, OpenFailure};
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent, MediaSource};
use greyscreen_core::settings::Settings;

const NVIDIA: u32 = 0x10de;
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const MAX_SESSIONS: usize = 64;
const WARNING_WAIT: Duration = Duration::from_secs(15);

fn config() -> CaptureConfig {
    Settings::default()
        .capture_config(HEIGHT)
        .for_encode_size(WIDTH, HEIGHT)
}

fn open_until_refused(gpu: &GpuDevice, cfg: &CaptureConfig) -> (Vec<H264Encoder>, Option<String>) {
    let mut held = Vec::new();
    while held.len() < MAX_SESSIONS {
        match H264Encoder::new(gpu, EncoderKind::Hardware, WIDTH, HEIGHT, cfg.fps, cfg) {
            Ok(encoder) => {
                eprintln!("session {} opened", held.len() + 1);
                held.push(encoder);
            }
            Err(error) => {
                let text = error.to_string();
                eprintln!("session {} refused: {text}", held.len() + 1);
                eprintln!("classified as {:?}", OpenFailure::from_text(&text));
                return (held, Some(text));
            }
        }
    }
    (held, None)
}

fn nvidia_gpu() -> Option<GpuDevice> {
    let adapter = adapters::for_target(&CaptureTarget::PrimaryMonitor)
        .into_iter()
        .find(|a| a.vendor_id == NVIDIA)?;
    GpuDevice::for_luid(adapter.luid).ok()
}

#[test]
#[ignore = "measures the NVIDIA session limit on the PC with no other encoder running"]
fn the_driver_refuses_a_session_after_its_limit() {
    let _com = ComGuard::mta().unwrap();
    let _mf = MfGuard::start().unwrap();
    let Some(gpu) = nvidia_gpu() else {
        eprintln!("no NVIDIA adapter, nothing to measure");
        return;
    };
    let (held, refusal) = open_until_refused(&gpu, &config());
    eprintln!("the driver allowed {} sessions", held.len());
    assert!(
        refusal.is_some(),
        "no refusal within {MAX_SESSIONS} sessions"
    );
}

#[test]
#[ignore = "holds every NVIDIA encoder session, then records on the primary monitor"]
fn a_busy_encoder_makes_a_recording_fall_back_with_the_busy_text() {
    let _com = ComGuard::mta().unwrap();
    let _mf = MfGuard::start().unwrap();
    let Some(gpu) = nvidia_gpu() else {
        eprintln!("no NVIDIA adapter, nothing to measure");
        return;
    };
    let (held, refusal) = open_until_refused(&gpu, &config());
    assert!(refusal.is_some(), "the driver never refused a session");
    let (tx, rx) = unbounded();
    let session = WindowsMediaSource::new()
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    let deadline = Instant::now() + WARNING_WAIT;
    let mut warnings = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(MediaEvent::Warning(text)) => warnings.push(text),
            Ok(MediaEvent::Started { encoder, .. }) => eprintln!("started with {encoder}"),
            Ok(_) => {}
            Err(_) => break,
        }
        if warnings.iter().any(|w| w == BUSY_HELP) {
            break;
        }
    }
    session.stop();
    drop(held);
    assert!(
        warnings.iter().any(|w| w == BUSY_HELP),
        "warnings were {warnings:?}"
    );
}
