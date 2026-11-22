use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::encoder::{self, EncoderKind};
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent};
use greyscreen_core::problem::Problem;
use windows::Win32::Foundation::E_FAIL;

use super::build;
use crate::adapters;
use crate::fault::{Armed, Point};
use crate::session::Flags;
use crate::session::video::Video;

const TARGET: CaptureTarget = CaptureTarget::PrimaryMonitor;

fn config() -> CaptureConfig {
    CaptureConfig {
        fps: 30,
        max_height: Some(360),
        video_bitrate: 4_000_000,
        video_peak: 6_000_000,
        quality: None,
        quality_vs_speed: None,
        gop_frames: 30,
        audio: false,
        audio_bitrate: 128_000,
        cursor: false,
        bitrate_pixels: 0,
        diagnostics: false,
    }
}

fn setup() -> (Result<Video, String>, Receiver<MediaEvent>) {
    let (tx, rx) = unbounded();
    (build(&TARGET, &config(), tx, Flags::default()), rx)
}

fn failure() -> String {
    match setup().0 {
        Ok(_) => panic!("setup should fail"),
        Err(message) => message,
    }
}

fn plan_entries() -> Vec<String> {
    let adapters = adapters::for_target(&TARGET);
    let plan = encoder::plan(&adapters);
    assert!(
        !plan.is_empty(),
        "the PC needs at least one encoder candidate"
    );
    plan.iter()
        .map(|candidate| {
            let adapter = adapters
                .iter()
                .find(|adapter| adapter.luid == candidate.adapter)
                .unwrap();
            let label = encoder::label(candidate.kind, adapter.vendor_id, &adapter.name);
            format!("{label} on {}", adapter.name)
        })
        .collect()
}

#[test]
fn setup_works_with_nothing_armed() {
    let _armed = Armed::new();
    assert!(setup().0.is_ok());
}

#[test]
fn a_gpu_that_cannot_open_gives_the_no_gpu_reason() {
    let _armed = Armed::new().fail(Point::AdapterOpen, E_FAIL);
    let message = failure();
    assert!(message.starts_with("no usable GPU"), "{message}");
}

#[test]
fn a_capture_start_failure_is_a_setup_failure_not_an_encoder_failure() {
    let _armed = Armed::new().fail(Point::CaptureStart, E_FAIL);
    let message = failure();
    assert!(
        message.starts_with("screen capture failed to start"),
        "{message}"
    );
    assert!(!message.contains("video encoder failed"));
}

#[test]
fn a_first_frame_failure_is_a_setup_failure_not_an_encoder_failure() {
    let _armed = Armed::new().fail(Point::FirstFrameWait, E_FAIL);
    let message = failure();
    assert!(!message.starts_with("video encoder failed"), "{message}");
    assert!(!message.starts_with("no usable GPU"), "{message}");
}

#[test]
fn a_converter_failure_names_every_encoder_and_adapter_tried() {
    let expected = plan_entries();
    let _armed = Armed::new().fail(Point::ConverterSetup, E_FAIL);
    let message = failure();
    assert!(message.starts_with("video encoder failed: "), "{message}");
    assert!(message.contains("video converter failed"), "{message}");
    for entry in expected {
        assert!(message.contains(&entry), "{entry} missing from {message}");
    }
}

#[test]
fn an_encoder_candidate_failure_names_every_encoder_and_adapter_tried() {
    let expected = plan_entries();
    let _armed = Armed::new().fail(Point::EncoderCandidate, E_FAIL);
    let message = failure();
    assert!(message.starts_with("video encoder failed: "), "{message}");
    assert!(!message.contains("video converter failed"), "{message}");
    for entry in expected {
        assert!(message.contains(&entry), "{entry} missing from {message}");
    }
}

#[test]
fn a_refused_hardware_encoder_falls_back_to_the_software_encoder() {
    let _armed = Armed::new().fail(Point::MftActivation, E_FAIL);
    let (result, _events) = setup();
    let Ok(video) = result else {
        panic!("the software encoder should take over");
    };
    assert_eq!(video.encoder.kind(), EncoderKind::Software);
}

#[test]
fn a_failed_setup_leaves_the_next_setup_able_to_succeed() {
    {
        let _armed = Armed::new().fail(Point::ConverterSetup, E_FAIL);
        failure();
    }
    {
        let _armed = Armed::new().fail(Point::CaptureStart, E_FAIL);
        failure();
    }
    let _armed = Armed::new();
    assert!(setup().0.is_ok());
}

#[test]
fn the_three_user_facing_reasons_stay_distinct() {
    let gpu = {
        let _armed = Armed::new().fail(Point::AdapterOpen, E_FAIL);
        failure()
    };
    let unsupported = {
        let _armed = Armed::new().fail(Point::ConversionSupport, E_FAIL);
        failure()
    };
    let encoder = {
        let _armed = Armed::new().fail(Point::EncoderCandidate, E_FAIL);
        failure()
    };
    assert!(gpu.starts_with("no usable GPU"), "{gpu}");
    assert!(unsupported.contains("BGRA to NV12"), "{unsupported}");
    assert!(!encoder.contains("video converter failed"), "{encoder}");
    let problems = [
        Problem::from_error(&gpu),
        Problem::from_error(&unsupported),
        Problem::from_error(&encoder),
    ];
    assert_ne!(problems[0], problems[1]);
    assert_ne!(problems[0], problems[2]);
    assert_ne!(problems[1], problems[2]);
}
