#![allow(clippy::expect_used)]

mod common;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use greyscreen_core::settings::Settings;
use greyscreen_core::update::{Asset, Release};
use greyscreen_engine::updater::{UpdateConfig, UpdateSource};
use greyscreen_engine::{Engine, EngineCmd, EngineConfig, EngineStatus, FakeMediaSource, UiMsg};
use greyscreen_league::UpdateError;

use common::{WAIT, no_league, recording_since, temp_dir, wait_for};

#[derive(Default)]
struct CountingSource {
    checks: AtomicUsize,
}

impl UpdateSource for CountingSource {
    fn releases(&self) -> Result<Vec<Release>, UpdateError> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    fn download(&self, _: &Asset, _: &Path) -> Result<(), UpdateError> {
        Ok(())
    }
}

#[test]
fn no_scheduled_check_runs_while_a_session_is_recording() {
    let source = Arc::new(CountingSource::default());
    let dir = temp_dir("updates");
    let settings = Settings {
        output_dir: dir.clone(),
        ..Settings::default()
    };
    let (tx, ui) = unbounded();
    let engine = Engine::start_with(EngineConfig {
        settings,
        media: Arc::new(FakeMediaSource::new()),
        league: no_league(),
        sink: Box::new(move |msg: UiMsg| {
            tx.send(msg).ok();
        }),
        settings_path: None,
        updates: Some(UpdateConfig {
            source: source.clone(),
            running: "0.1.6-preview.8".into(),
            can_install: true,
            dir: dir.join("downloads"),
            launch: Box::new(|_| Ok(())),
            first_check: Duration::from_millis(300),
            interval: Duration::from_millis(100),
            busy_retry: Duration::from_millis(100),
        }),
        feedback: None,
        hotkeys: None,
    });
    engine
        .cmd
        .send(EngineCmd::StartTestRecording { secs: 2 })
        .expect("engine listening");
    recording_since(&ui);
    std::thread::sleep(Duration::from_millis(1200));
    assert_eq!(source.checks.load(Ordering::SeqCst), 0);
    wait_for(&ui, |msg| {
        matches!(msg, UiMsg::Status(EngineStatus::Idle)).then_some(())
    })
    .expect("recording ends");
    assert_eq!(source.checks.load(Ordering::SeqCst), 0);
    let deadline = Instant::now() + WAIT;
    while source.checks.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(source.checks.load(Ordering::SeqCst) >= 1);
    drop(engine);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_manual_check_without_an_updater_says_so() {
    let dir = temp_dir("updates-none");
    let (tx, ui) = unbounded();
    let engine = Engine::start_with(EngineConfig {
        settings: Settings {
            output_dir: dir.clone(),
            ..Settings::default()
        },
        media: Arc::new(FakeMediaSource::new()),
        league: no_league(),
        sink: Box::new(move |msg: UiMsg| {
            tx.send(msg).ok();
        }),
        settings_path: None,
        updates: None,
        feedback: None,
        hotkeys: None,
    });
    engine
        .cmd
        .send(EngineCmd::CheckForUpdates)
        .expect("engine listening");
    let state = wait_for(&ui, |msg| match msg {
        UiMsg::Update(state) => Some(state.clone()),
        _ => None,
    })
    .expect("an update message");
    assert!(matches!(
        state,
        greyscreen_core::update::UpdateState::Failed(_)
    ));
    drop(engine);
    std::fs::remove_dir_all(dir).ok();
}
