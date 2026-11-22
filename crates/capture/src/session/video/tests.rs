use std::time::Duration;

use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::encoder::is_encoder_failure;
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent};
use windows::Win32::Foundation::E_FAIL;
use windows::Win32::Graphics::Dxgi::{DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET};
use windows::core::{Error, HRESULT};

use super::{Exit, MAX_SOFT_ERRORS, Video, lost_h264, lost_vproc};
use crate::fault::{Armed, Point};
use crate::h264::H264Error;
use crate::session::Flags;
use crate::session::setup::build;
use crate::vproc::VprocError;

const LOSSES: [HRESULT; 2] = [DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET];

fn config() -> CaptureConfig {
    CaptureConfig {
        fps: 30,
        max_height: Some(360),
        video_bitrate: 4_000_000,
        video_peak: 6_000_000,
        quality: None,
        gop_frames: 30,
        audio: false,
        audio_bitrate: 128_000,
        cursor: false,
        bitrate_pixels: 0,
        diagnostics: false,
    }
}

fn video() -> (Video, Receiver<MediaEvent>) {
    let (tx, rx) = unbounded();
    match build(
        &CaptureTarget::PrimaryMonitor,
        &config(),
        tx,
        Flags::default(),
    ) {
        Ok(video) => (video, rx),
        Err(message) => panic!("setup failed: {message}"),
    }
}

fn lost_reason(exit: Exit) -> String {
    match exit {
        Exit::Lost(reason) => reason,
        _ => panic!("expected the lost exit"),
    }
}

fn assert_ended_with_one_failure(events: &Receiver<MediaEvent>, prefix: &str) {
    let events: Vec<MediaEvent> = events.try_iter().collect();
    let failures: Vec<&String> = events
        .iter()
        .filter_map(|event| match event {
            MediaEvent::Failed(reason) => Some(reason),
            _ => None,
        })
        .collect();
    assert_eq!(failures.len(), 1, "{events:?}");
    assert!(failures[0].starts_with(prefix), "{}", failures[0]);
    assert!(is_encoder_failure(failures[0]));
    assert!(matches!(events.last(), Some(MediaEvent::Failed(_))));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, MediaEvent::Packet(_)))
    );
}

fn packet_count(events: &[MediaEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, MediaEvent::Packet(_)))
        .count()
}

fn lose_device_mid_recording(point: Point, code: HRESULT, prefix: &str) {
    let armed = Armed::new();
    let (tx, rx) = unbounded();
    let flags = Flags::default();
    let stop = flags.stop.clone();
    let runner = std::thread::spawn(move || {
        match build(&CaptureTarget::PrimaryMonitor, &config(), tx, flags) {
            Ok(video) => video.run(),
            Err(message) => panic!("setup failed: {message}"),
        }
    });
    let mut seen = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(event @ MediaEvent::Packet(_)) => {
                seen.push(event);
                break;
            }
            Ok(event) => seen.push(event),
            Err(_) => {
                stop.store(true, std::sync::atomic::Ordering::Release);
                panic!("no packet before the loss: {seen:?}");
            }
        }
    }
    let _armed = armed.fail(point, code);
    runner.join().unwrap();
    seen.extend(rx.try_iter());
    assert!(packet_count(&seen) >= 1);
    let failed: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter_map(|(index, event)| matches!(event, MediaEvent::Failed(_)).then_some(index))
        .collect();
    assert_eq!(failed.len(), 1, "{seen:?}");
    let Some(MediaEvent::Failed(reason)) = seen.get(failed[0]) else {
        unreachable!()
    };
    assert!(reason.starts_with(prefix), "{reason}");
    assert!(is_encoder_failure(reason));
    assert_eq!(failed[0], seen.len() - 1, "an event follows the loss");
}

#[test]
fn a_device_loss_in_the_check_after_packets_ends_the_run_with_one_failure() {
    for code in LOSSES {
        lose_device_mid_recording(Point::DeviceRemovedCheck, code, "graphics device removed");
    }
}

#[test]
fn a_device_loss_inside_convert_after_packets_ends_the_run_with_one_failure() {
    for code in LOSSES {
        lose_device_mid_recording(
            Point::ConverterConvert,
            code,
            "graphics device lost in convert",
        );
    }
}

#[test]
fn a_device_loss_inside_submit_after_packets_ends_the_run_with_one_failure() {
    for code in LOSSES {
        lose_device_mid_recording(
            Point::EncoderSubmit,
            code,
            "graphics device lost in encoder submit",
        );
    }
}

#[test]
fn a_device_removed_or_reset_from_the_check_ends_the_run_with_the_lost_exit() {
    for code in LOSSES {
        let _armed = Armed::new().fail(Point::DeviceRemovedCheck, code);
        let (video, events) = video();
        video.run();
        assert_ended_with_one_failure(&events, "graphics device removed");
    }
}

#[test]
fn the_converter_loss_classification_gives_the_lost_exit() {
    for code in LOSSES {
        let _armed = Armed::new();
        let (mut video, events) = video();
        let error = VprocError::Windows(Error::from(code));
        assert!(lost_vproc(&error));
        let exit = video
            .soft("convert", &error, lost_vproc(&error))
            .unwrap_err();
        let reason = lost_reason(exit);
        assert!(reason.starts_with("graphics device lost in convert"));
        video.shutdown(Exit::Lost(reason));
        assert_ended_with_one_failure(&events, "graphics device lost in convert");
    }
}

#[test]
fn the_encoder_loss_classification_gives_the_lost_exit() {
    for code in LOSSES {
        let _armed = Armed::new();
        let (mut video, events) = video();
        let error = H264Error::Windows(Error::from(code));
        assert!(lost_h264(&error));
        let exit = video.encoder_error("encoder submit", &error).unwrap_err();
        let reason = lost_reason(exit);
        assert!(reason.starts_with("graphics device lost in encoder submit"));
        video.shutdown(Exit::Lost(reason));
        assert_ended_with_one_failure(&events, "graphics device lost in encoder submit");
    }
}

#[test]
fn an_encoder_that_reports_an_error_ends_with_the_stopped_reason() {
    let _armed = Armed::new();
    let (mut video, events) = video();
    let error = H264Error::Stopped(Error::from(DXGI_ERROR_DEVICE_REMOVED));
    let Err(Exit::Failed(reason)) = video.encoder_error("encoder drain", &error) else {
        panic!("expected the failed exit");
    };
    video.shutdown(Exit::Failed(reason));
    assert_ended_with_one_failure(&events, "the video encoder stopped");
}

#[test]
fn an_ordinary_step_error_is_soft_until_it_keeps_repeating() {
    let _armed = Armed::new();
    let (mut video, _events) = video();
    let error = VprocError::Windows(Error::from(E_FAIL));
    assert!(!lost_vproc(&error));
    for _ in 1..MAX_SOFT_ERRORS {
        assert!(video.soft("convert", &error, false).is_ok());
    }
    assert!(matches!(
        video.soft("convert", &error, false),
        Err(Exit::Failed(_))
    ));
}
