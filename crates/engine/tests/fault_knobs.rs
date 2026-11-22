#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    EngineCmd, EngineStatus, EngineTiming, FakeMediaSource, LeagueFactory, UiMsg,
};
use greyscreen_league::LeagueMsg;

use common::{info, recording_since, start_engine_timed, start_idle, temp_dir, wait_for};

fn settings(tag: &str) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        ..Settings::default()
    }
}

struct FailsFirstStart {
    inner: FakeMediaSource,
    failed: AtomicBool,
}

impl MediaSource for FailsFirstStart {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        if !self.failed.swap(true, Ordering::SeqCst) {
            return Err("no gpu".to_owned());
        }
        self.inner.start(target, cfg, out)
    }
}

fn ready_game() -> LeagueFactory {
    Box::new(|out: Sender<LeagueMsg>| {
        out.send(LeagueMsg::WindowFound { hwnd: 7, pid: 42 }).ok();
        out.send(LeagueMsg::Ready(info())).ok();
        Box::new(out)
    })
}

#[test]
fn a_failed_start_reaches_the_user_and_leaves_no_live_session() {
    let media = FakeMediaSource::new().failing_to_start("boom: no encoder");
    let probe = media.probe();
    let harness = start_idle(settings("knob-start"), Arc::new(media));
    harness
        .engine
        .as_ref()
        .expect("engine")
        .cmd
        .send(EngineCmd::StartTestRecording { secs: 1 })
        .expect("send");
    let error = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Status(EngineStatus::Error(text)) => Some(text.clone()),
        _ => None,
    })
    .expect("error status");
    assert!(error.contains("boom: no encoder"), "{error}");
    assert_eq!(probe.live_sessions(), 0);
}

#[test]
fn live_sessions_return_to_zero_after_the_recording_is_saved() {
    let media = FakeMediaSource::new();
    let probe = media.probe();
    let harness = start_idle(settings("knob-live"), Arc::new(media));
    harness
        .engine
        .as_ref()
        .expect("engine")
        .cmd
        .send(EngineCmd::StartTestRecording { secs: 1 })
        .expect("send");
    recording_since(&harness.ui);
    assert_eq!(probe.live_sessions(), 1);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::RecordingSaved(_)).then_some(())
    })
    .expect("recording saved");
    let deadline = Instant::now() + Duration::from_secs(5);
    while probe.live_sessions() > 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(probe.live_sessions(), 0);
}

#[test]
fn a_short_shutdown_grace_bounds_a_blocked_stop() {
    let media = FakeMediaSource::new().blocking_stop(Duration::from_secs(1));
    let timing = EngineTiming {
        shutdown_grace: Duration::from_millis(200),
        ..EngineTiming::default()
    };
    let mut harness = start_engine_timed(
        settings("knob-grace"),
        Arc::new(media),
        common::no_league(),
        timing,
    );
    harness
        .engine
        .as_ref()
        .expect("engine")
        .cmd
        .send(EngineCmd::StartTestRecording { secs: 60 })
        .expect("send");
    recording_since(&harness.ui);
    let began = Instant::now();
    drop(harness.engine.take());
    let took = began.elapsed();
    assert!(took < Duration::from_secs(1), "{took:?}");
}

#[test]
fn a_short_retry_delay_brings_the_retry_within_two_seconds() {
    let media = FailsFirstStart {
        inner: FakeMediaSource::new(),
        failed: AtomicBool::new(false),
    };
    let timing = EngineTiming {
        retry_delay: Duration::from_secs(1),
        ..EngineTiming::default()
    };
    let began = Instant::now();
    let harness = start_engine_timed(
        settings("knob-retry"),
        Arc::new(media),
        ready_game(),
        timing,
    );
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })).then_some(())
    })
    .expect("recording after the retry");
    let took = began.elapsed();
    assert!(took >= Duration::from_secs(1), "{took:?}");
    assert!(took < Duration::from_secs(2), "{took:?}");
}
