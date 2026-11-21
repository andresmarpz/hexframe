use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{Env, Runtime};

use crate::ui::{Screen, Ui, parse_clip_secs};
use crate::view;

fn runtime_with(ui: Ui) -> Runtime<Ui> {
    let mut rt = Runtime::new(ui, view::root, (960.0, 640.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn runtime() -> Runtime<Ui> {
    runtime_with(Ui::preview())
}

fn live_runtime() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (cmd, rx) = unbounded();
    let mut ui = Ui::live(Settings::default(), cmd);
    ui.recordings = Ui::preview().recordings;
    (runtime_with(ui), rx)
}

fn tap(rt: &mut Runtime<Ui>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

#[test]
fn nav_buttons_switch_the_screen() {
    let mut rt = runtime();
    assert_eq!(rt.state.screen, Screen::Status);
    tap(&mut rt, "nav-recordings");
    assert_eq!(rt.state.screen, Screen::Recordings);
    tap(&mut rt, "nav-settings");
    assert_eq!(rt.state.screen, Screen::Settings);
    tap(&mut rt, "nav-status");
    assert_eq!(rt.state.screen, Screen::Status);
}

#[test]
fn test_recording_button_sends_the_command() {
    let (mut rt, rx) = live_runtime();
    tap(&mut rt, "test-recording");
    assert_eq!(
        rx.try_recv(),
        Ok(EngineCmd::StartTestRecording { secs: 10 })
    );
}

#[test]
fn test_recording_button_is_disabled_unless_idle() {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| ui.status = EngineStatus::Waiting);
    rt.frame(1.0 / 60.0);
    tap(&mut rt, "test-recording");
    assert!(rx.try_recv().is_err());
}

#[test]
fn stop_and_save_clip_buttons_send_commands_while_recording() {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| ui.status = Ui::preview().status);
    rt.frame(1.0 / 60.0);
    tap(&mut rt, "save-clip");
    tap(&mut rt, "stop-recording");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::SaveClipNow));
    assert_eq!(rx.try_recv(), Ok(EngineCmd::StopRecording));
}

#[test]
fn opening_recordings_asks_the_engine_to_rescan() {
    let (mut rt, rx) = live_runtime();
    tap(&mut rt, "nav-recordings");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::RescanLibrary));
}

#[test]
fn review_button_opens_the_review_screen_with_the_id() {
    let (mut rt, _rx) = live_runtime();
    tap(&mut rt, "nav-recordings");
    let id = rt.state.recordings[0].meta.id.clone();
    tap(&mut rt, &format!("review-{id}"));
    assert_eq!(rt.state.screen, Screen::Review);
    assert_eq!(rt.state.review, Some(id));
    tap(&mut rt, "review-back");
    assert_eq!(rt.state.screen, Screen::Recordings);
}

#[test]
fn toggling_a_switch_applies_the_setting_through_the_engine() {
    let (mut rt, rx) = live_runtime();
    tap(&mut rt, "nav-settings");
    let before = rt.state.settings.audio;
    tap(&mut rt, "audio-switch");
    assert_eq!(rt.state.settings.audio, !before);
    let Ok(EngineCmd::ApplySettings(sent)) = rx.try_recv() else {
        panic!("no ApplySettings sent");
    };
    assert_eq!(sent.audio, !before);
}

#[test]
fn full_game_switch_changes_the_setting() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    let before = rt.state.settings.record_full_game;
    tap(&mut rt, "full-game-switch");
    assert_eq!(rt.state.settings.record_full_game, !before);
}

#[test]
fn picking_a_segment_changes_the_setting() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    assert_eq!(rt.state.settings.fps, 60);
    let (x, y) = rt.find("fps").unwrap();
    rt.pointer_down(x - 30.0, y);
    rt.pointer_up(x - 30.0, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.settings.fps, 30);
}

#[test]
fn clip_seconds_outside_the_range_are_ignored() {
    assert_eq!(parse_clip_secs("15"), Some(15));
    assert_eq!(parse_clip_secs(" 120 "), Some(120));
    assert_eq!(parse_clip_secs("0"), None);
    assert_eq!(parse_clip_secs("121"), None);
    assert_eq!(parse_clip_secs("abc"), None);
}
