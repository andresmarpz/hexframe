#![cfg(windows)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crossbeam_channel::unbounded;
use greyscreen_capture::fault::{Armed, Point};
use greyscreen_capture::{WindowsMediaSource, live_sessions};
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaSource};

const STALL: Duration = Duration::from_secs(13);
static SERIAL: Mutex<()> = Mutex::new(());

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

fn start() -> Result<Box<dyn greyscreen_core::media::MediaSession>, String> {
    let (tx, rx) = unbounded();
    std::mem::forget(rx);
    WindowsMediaSource::new().start(CaptureTarget::PrimaryMonitor, config(), tx)
}

#[test]
#[ignore = "needs the PC: stalls capture setup past its 10 s bound"]
fn a_timed_out_setup_exits_when_it_wakes_and_the_next_start_works() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let armed = Armed::new().stall(Point::CaptureStart, STALL);
    let error = start().err().expect("setup must time out");
    assert!(error.contains("timed out"), "{error}");
    assert_eq!(live_sessions(), 1, "the stalled thread is still running");
    drop(armed);
    let session = start().expect("the next start works");
    assert_eq!(
        live_sessions(),
        1,
        "the next start overlapped the stalled thread"
    );
    session.stop();
    assert_eq!(live_sessions(), 0);
}

#[test]
#[ignore = "needs the PC: stalls capture setup past its 10 s bound"]
fn two_starts_in_a_row_never_have_two_live_sessions() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let armed = Armed::new().stall(Point::CaptureStart, STALL);
    start().err().expect("setup must time out");
    drop(armed);
    let peak = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let watcher = {
        let (peak, done) = (peak.clone(), done.clone());
        std::thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                peak.fetch_max(live_sessions(), Ordering::AcqRel);
                std::thread::sleep(Duration::from_millis(5));
            }
        })
    };
    let second = start();
    done.store(true, Ordering::Release);
    watcher.join().expect("watcher");
    assert!(peak.load(Ordering::Acquire) <= 1, "two sessions were live");
    match second {
        Ok(session) => session.stop(),
        Err(message) => assert!(message.contains("still shutting down"), "{message}"),
    }
}
