use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource};

use super::setup::build;
use super::{Flags, Session, WindowsMediaSource};
use crate::fault::Armed;
use crate::h264::H264Encoder;
use crate::testing::fake_transform::FakeTransform;

const HELD_STOP_TIMEOUT: Duration = Duration::from_millis(300);
const FLUSH_BLOCK: Duration = Duration::from_millis(1500);

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

#[test]
fn a_video_thread_held_past_the_stop_timeout_does_not_block_the_next_start() {
    let _armed = Armed::new();
    let release = Arc::new(AtomicBool::new(false));
    let held = Arc::clone(&release);
    let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(0);
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<()>(1);
    let thread = std::thread::spawn(move || {
        let _done = done_tx;
        let (tx, _rx) = unbounded();
        let video = match build(
            &CaptureTarget::PrimaryMonitor,
            &config(),
            tx,
            Flags::default(),
        ) {
            Ok(video) => video,
            Err(message) => panic!("setup failed: {message}"),
        };
        ready_tx.send(()).ok();
        while !held.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(video);
    });
    ready_rx
        .recv_timeout(Duration::from_secs(20))
        .expect("the held thread opened its encoder");
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

#[test]
fn a_flush_that_outlasts_the_stop_timeout_does_not_hold_stop_and_keeps_the_early_packets() {
    let _armed = Armed::new();
    let (tx, rx) = unbounded();
    let flags = Flags::default();
    let thread_flags = flags.clone();
    let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(0);
    let thread = std::thread::spawn(move || {
        let _done = done_tx;
        let mut video = match build(&CaptureTarget::PrimaryMonitor, &config(), tx, thread_flags) {
            Ok(video) => video,
            Err(message) => panic!("setup failed: {message}"),
        };
        let fake = FakeTransform::new().blocking_flush(FLUSH_BLOCK);
        video.encoder = H264Encoder::on_fake(fake, 64, 64)
            .unwrap()
            .with_releaser(video.converter.releaser());
        video.run();
    });
    let waiter = done_rx.clone();
    let mut early = 0;
    while early == 0 {
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(MediaEvent::Packet(_)) => early += 1,
            Ok(_) => {}
            Err(_) => {
                flags.stop.store(true, Ordering::Release);
                panic!("no packet before the stop");
            }
        }
    }
    let session = Session {
        thread: Some(thread),
        flags,
        done: done_rx,
        stop_timeout: HELD_STOP_TIMEOUT,
    };
    let started = Instant::now();
    Box::new(session).stop();
    assert!(started.elapsed() < HELD_STOP_TIMEOUT + Duration::from_millis(500));
    assert!(early >= 1);
    assert!(waiter.recv_timeout(Duration::from_secs(20)).is_err());
}
