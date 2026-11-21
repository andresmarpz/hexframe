#![allow(clippy::expect_used)]

mod common;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use greyscreen_core::keybinds::{Action, KeyCombo};
use greyscreen_core::settings::{ClipSettings, Settings};
use greyscreen_engine::{Engine, EngineCmd, EngineConfig, FakeMediaSource, HotkeyBackend, UiMsg};

use common::{recording_since, start_idle, temp_dir, wait_for};

fn settings(tag: &str) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        clips: ClipSettings {
            enabled: false,
            ..ClipSettings::default()
        },
        ..Settings::default()
    }
}

#[test]
fn the_hotkey_saves_a_clip_while_recording() {
    let harness = start_idle(settings("hotkey-on"), Arc::new(FakeMediaSource::new()));
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 4 }).ok();
    recording_since(&harness.ui);
    thread::sleep(Duration::from_millis(1500));
    cmd.send(EngineCmd::GlobalAction(Action::SaveClip)).ok();
    let clip = wait_for(&harness.ui, |msg| match msg {
        UiMsg::ClipSaved(clip) => Some(clip.clone()),
        _ => None,
    })
    .expect("clip saved");
    assert_eq!(clip.clip.label, "Manual");
    assert!(clip.folder.join(&clip.clip.file).is_file());
}

#[test]
fn the_hotkey_does_nothing_when_nothing_is_recording() {
    let harness = start_idle(settings("hotkey-idle"), Arc::new(FakeMediaSource::new()));
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::GlobalAction(Action::SaveClip)).ok();
    thread::sleep(Duration::from_millis(600));
    let seen: Vec<UiMsg> = harness.ui.try_iter().collect();
    assert!(
        seen.iter()
            .all(|msg| !matches!(msg, UiMsg::Notice { .. } | UiMsg::ClipSaved(_))),
        "{seen:?}"
    );
}

#[test]
fn the_hotkey_does_nothing_after_the_recording_ends() {
    let harness = start_idle(settings("hotkey-after"), Arc::new(FakeMediaSource::new()));
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 1 }).ok();
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::RecordingSaved(_)).then_some(())
    })
    .expect("recording saved");
    cmd.send(EngineCmd::GlobalAction(Action::SaveClip)).ok();
    thread::sleep(Duration::from_millis(600));
    let seen: Vec<UiMsg> = harness.ui.try_iter().collect();
    assert!(
        seen.iter()
            .all(|msg| !matches!(msg, UiMsg::Notice { .. } | UiMsg::ClipSaved(_))),
        "{seen:?}"
    );
}

type Registration = Vec<(Action, KeyCombo)>;

#[derive(Clone, Default)]
struct FakeHotkeys {
    calls: Arc<Mutex<Vec<Registration>>>,
}

impl HotkeyBackend for FakeHotkeys {
    fn set(&self, bindings: &[(Action, KeyCombo)]) {
        self.calls.lock().expect("calls").push(bindings.to_vec());
    }
}

fn start_with_fake(settings: Settings) -> (common::Harness, FakeHotkeys) {
    let fake = FakeHotkeys::default();
    let dir = settings.output_dir.clone();
    let settings_path = dir.with_extension("settings.json");
    let (tx, rx) = crossbeam_channel::unbounded();
    let engine = Engine::start_with(EngineConfig {
        settings,
        media: Arc::new(FakeMediaSource::new()),
        league: common::no_league(),
        sink: Box::new(move |msg| {
            tx.send(msg).ok();
        }),
        settings_path: Some(settings_path.clone()),
        updates: None,
        feedback: None,
        hotkeys: Some(Box::new(fake.clone())),
    });
    (
        common::Harness {
            engine: Some(engine),
            ui: rx,
            dir,
            settings_path,
        },
        fake,
    )
}

fn wait_until(what: &str, check: impl Fn() -> bool) {
    for _ in 0..200 {
        if check() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {what}");
}

fn combo(text: &str) -> KeyCombo {
    text.parse().expect("combo")
}

#[test]
fn the_global_bindings_are_registered_at_startup() {
    let (_harness, fake) = start_with_fake(settings("hotkey-start"));
    wait_until("the startup registration", || {
        !fake.calls.lock().expect("calls").is_empty()
    });
    assert_eq!(
        fake.calls.lock().expect("calls")[0],
        vec![(Action::SaveClip, combo("F9"))]
    );
}

#[test]
fn a_rebound_global_key_is_registered_and_an_unchanged_one_is_not() {
    let (harness, fake) = start_with_fake(settings("hotkey-rebind"));
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    let mut changed = settings("hotkey-rebind");
    changed
        .keybinds
        .insert("save_clip".into(), "Ctrl+Shift+K".into());
    cmd.send(EngineCmd::ApplySettings(changed.clone())).ok();
    wait_until("the new registration", || {
        fake.calls.lock().expect("calls").len() == 2
    });
    changed.audio = !changed.audio;
    cmd.send(EngineCmd::ApplySettings(changed.clone())).ok();
    changed.keybinds.insert("save_clip".into(), "none".into());
    cmd.send(EngineCmd::ApplySettings(changed)).ok();
    wait_until("the clearing registration", || {
        fake.calls.lock().expect("calls").len() == 3
    });
    let calls = fake.calls.lock().expect("calls").clone();
    assert_eq!(calls[1], vec![(Action::SaveClip, combo("Ctrl+Shift+K"))]);
    assert!(calls[2].is_empty());
}

#[test]
fn an_app_key_change_does_not_touch_the_registration() {
    let (harness, fake) = start_with_fake(settings("hotkey-app"));
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    wait_until("the startup registration", || {
        !fake.calls.lock().expect("calls").is_empty()
    });
    let mut changed = settings("hotkey-app");
    changed.keybinds.insert("mute".into(), "N".into());
    cmd.send(EngineCmd::ApplySettings(changed)).ok();
    thread::sleep(Duration::from_millis(400));
    assert_eq!(fake.calls.lock().expect("calls").len(), 1);
}
