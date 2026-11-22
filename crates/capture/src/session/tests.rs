use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaSession, MediaSource};

use super::{Flags, Session, WindowsMediaSource};
use crate::fault::Armed;

const HELD_STOP_TIMEOUT: Duration = Duration::from_millis(300);

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

#[test]
fn a_video_thread_held_past_the_stop_timeout_does_not_block_the_next_start() {
    let _armed = Armed::new();
    let release = Arc::new(AtomicBool::new(false));
    let held = Arc::clone(&release);
    let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(0);
    let thread = std::thread::spawn(move || {
        let _done = done_tx;
        while !held.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let session = Session {
        thread: Some(thread),
        flags: Flags::default(),
        done: done_rx,
        stop_timeout: HELD_STOP_TIMEOUT,
    };
    let started = Instant::now();
    Box::new(session).stop();
    assert!(started.elapsed() >= HELD_STOP_TIMEOUT);
    assert!(started.elapsed() < HELD_STOP_TIMEOUT + Duration::from_secs(2));

    let (tx, _rx) = unbounded();
    let next = WindowsMediaSource::new().start(CaptureTarget::PrimaryMonitor, config(), tx);
    release.store(true, Ordering::Release);
    next.expect("the next start opens an encoder").stop();
}
