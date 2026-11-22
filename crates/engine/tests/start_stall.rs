#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    EngineCmd, EngineStatus, EngineTiming, FakeMediaSource, FakeProbe, LeagueFactory, UiMsg,
};
use greyscreen_league::LeagueMsg;

use common::{info, recording_since, start_engine_timed, start_idle, temp_dir, wait_for};

fn settings(tag: &str) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        ..Settings::default()
    }
}

fn ready_game() -> LeagueFactory {
    Box::new(|out: Sender<LeagueMsg>| {
        out.send(LeagueMsg::WindowFound { hwnd: 7, pid: 42 }).ok();
        out.send(LeagueMsg::Ready(info())).ok();
        Box::new(out)
    })
}

#[derive(Default)]
struct StartLog {
    calls: Vec<(Instant, Instant)>,
}

struct StallsFirstStart {
    inner: FakeMediaSource,
    stall: Duration,
    stalled: AtomicBool,
    log: Arc<Mutex<StartLog>>,
}

impl StallsFirstStart {
    fn new(stall: Duration) -> (Self, Arc<FakeProbe>, Arc<Mutex<StartLog>>) {
        let inner = FakeMediaSource::new();
        let probe = inner.probe();
        let log = Arc::new(Mutex::new(StartLog::default()));
        let source = Self {
            inner,
            stall,
            stalled: AtomicBool::new(false),
            log: log.clone(),
        };
        (source, probe, log)
    }
}

impl MediaSource for StallsFirstStart {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        let began = Instant::now();
        if !self.stalled.swap(true, Ordering::SeqCst) {
            std::thread::sleep(self.stall);
        }
        let started = self.inner.start(target, cfg, out);
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .calls
            .push((began, Instant::now()));
        started
    }
}

#[test]
fn a_library_rescan_is_answered_while_the_start_is_blocked() {
    let media = FakeMediaSource::new().blocking_start(Duration::from_secs(3));
    let harness = start_idle(settings("stall-rescan"), Arc::new(media));
    std::thread::sleep(Duration::from_millis(300));
    while harness.ui.try_recv().is_ok() {}
    let cmd = &harness.engine.as_ref().expect("engine").cmd;
    cmd.send(EngineCmd::StartTestRecording { secs: 60 })
        .expect("send");
    let sent = Instant::now();
    cmd.send(EngineCmd::RescanLibrary).expect("send");
    harness
        .ui
        .iter()
        .find(|msg| matches!(msg, UiMsg::Library(_)))
        .expect("library");
    let took = sent.elapsed();
    assert!(took < Duration::from_millis(100), "{took:?}");
}

#[test]
fn a_start_that_returns_late_is_stopped_and_the_retry_waits_for_it() {
    let (media, probe, log) = StallsFirstStart::new(Duration::from_millis(800));
    let timing = EngineTiming {
        media_start_deadline: Duration::from_millis(200),
        retry_delay: Duration::from_millis(100),
        ..EngineTiming::default()
    };
    let harness = start_engine_timed(
        settings("stall-late"),
        Arc::new(media),
        ready_game(),
        timing,
    );
    let failure = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice { text, .. } if text.contains("Capture failed to start") => Some(()),
        _ => None,
    });
    assert!(failure.is_some(), "no start failure notice");
    recording_since(&harness.ui);
    assert_eq!(probe.peak_live_sessions(), 1);
    let calls = log.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(calls.calls.len(), 2);
    assert!(
        calls.calls[1].0 >= calls.calls[0].1,
        "the retry began before the stalled start returned"
    );
}

#[test]
fn a_retry_waits_for_a_blocked_stop_of_a_late_session() {
    let (mut media, _probe, _log) = StallsFirstStart::new(Duration::from_millis(300));
    media.inner = media.inner.blocking_stop(Duration::from_secs(1));
    let probe = media.inner.probe();
    let timing = EngineTiming {
        media_start_deadline: Duration::from_millis(200),
        retry_delay: Duration::from_millis(300),
        ..EngineTiming::default()
    };
    let harness = start_engine_timed(
        settings("stall-blocked-stop"),
        Arc::new(media),
        ready_game(),
        timing,
    );
    recording_since(&harness.ui);
    assert_eq!(probe.peak_live_sessions(), 1);
}

#[test]
fn a_late_start_with_no_retry_leaves_no_session_running() {
    let (media, probe, _log) = StallsFirstStart::new(Duration::from_millis(600));
    let timing = EngineTiming {
        media_start_deadline: Duration::from_millis(150),
        ..EngineTiming::default()
    };
    let harness = start_engine_timed(
        settings("stall-none"),
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
    let error = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Status(EngineStatus::Error(text)) => Some(text.clone()),
        _ => None,
    });
    assert!(error.is_some());
    std::thread::sleep(Duration::from_millis(900));
    assert_eq!(probe.live_sessions(), 0);
    assert!(
        !harness
            .ui
            .try_iter()
            .any(|msg| matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })))
    );
}

#[test]
fn shutdown_during_a_blocked_start_stays_within_the_grace_and_leaves_no_session() {
    let media = FakeMediaSource::new().blocking_start(Duration::from_millis(700));
    let probe = media.probe();
    let timing = EngineTiming {
        shutdown_grace: Duration::from_secs(3),
        ..EngineTiming::default()
    };
    let mut harness = start_engine_timed(
        settings("stall-shutdown"),
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
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Status(EngineStatus::Waiting)).then_some(())
    })
    .expect("waiting");
    let began = Instant::now();
    drop(harness.engine.take());
    let took = began.elapsed();
    assert!(took < Duration::from_secs(3), "{took:?}");
    assert!(took >= Duration::from_millis(600), "{took:?}");
    assert_eq!(probe.peak_live_sessions(), 1);
    assert_eq!(probe.live_sessions(), 0);
}
