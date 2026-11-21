#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use greyscreen_core::settings::{ClipSettings, Settings};
use greyscreen_engine::{EngineCmd, FakeMediaSource, UiMsg};

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
    cmd.send(EngineCmd::HotkeyPressed).ok();
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
    cmd.send(EngineCmd::HotkeyPressed).ok();
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
    cmd.send(EngineCmd::HotkeyPressed).ok();
    thread::sleep(Duration::from_millis(600));
    let seen: Vec<UiMsg> = harness.ui.try_iter().collect();
    assert!(
        seen.iter()
            .all(|msg| !matches!(msg, UiMsg::Notice { .. } | UiMsg::ClipSaved(_))),
        "{seen:?}"
    );
}
