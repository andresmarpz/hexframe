use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{Env, Runtime};

use crate::ui::fake_player::{self, Call};
use crate::ui::{Screen, Source, Ui, parse_clip_secs};
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
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.entry.meta.id.clone()),
        Some(id)
    );
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

fn review_runtime() -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_with(ui);
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    fake_player::clear_calls();
    rt
}

fn assert_seeks_to(secs: f64) {
    match fake_player::calls().as_slice() {
        [Call::Seek(at)] => assert!((at - secs).abs() < 1e-9, "sought {at}"),
        other => panic!("unexpected calls {other:?}"),
    }
}

#[test]
fn tapping_a_marker_seeks_three_seconds_before_the_event() {
    let mut rt = review_runtime();
    tap(&mut rt, "marker-9");
    assert_seeks_to(5.2);
}

#[test]
fn pressing_the_track_seeks_to_that_share_of_the_video() {
    let mut rt = review_runtime();
    let (x, y) = rt.find("timeline").expect("timeline");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert_seeks_to(5.0);
}

#[test]
fn tapping_an_event_row_seeks_like_its_marker() {
    let mut rt = review_runtime();
    tap(&mut rt, "event-5");
    assert_seeks_to(1.6);
}

#[test]
fn tapping_the_play_button_plays_a_paused_player() {
    let mut rt = review_runtime();
    tap(&mut rt, "review-play");
    assert_eq!(fake_player::calls(), vec![Call::Play]);
}

#[test]
fn tapping_a_clip_swaps_the_source_and_back_restores_the_game() {
    let mut rt = review_runtime();
    tap(&mut rt, "clip-1");
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.source),
        Some(Source::Clip(1))
    );
    assert!(matches!(fake_player::calls()[0], Call::Open(_)));
    tap(&mut rt, "review-game");
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.source),
        Some(Source::Game)
    );
}

#[test]
fn leaving_the_review_screen_drops_the_player() {
    let mut rt = review_runtime();
    tap(&mut rt, "review-back");
    assert!(rt.state.review.is_none());
}

#[test]
fn the_resolution_menu_opens_under_its_trigger_and_picks_a_value() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    tap(&mut rt, "resolution-trigger");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.state.resolution_open);
    let (trigger, _) = rt.presented("resolution-trigger").expect("trigger");
    let (menu, _) = rt.presented("resolution-content").expect("menu");
    let trigger_bottom = trigger[1] + trigger[3];
    assert!(
        (menu[1] - trigger_bottom).abs() < 12.0,
        "menu top {} should sit under the trigger bottom {trigger_bottom}",
        menu[1]
    );
    let trigger_right = trigger[0] + trigger[2];
    let menu_right = menu[0] + menu[2];
    assert!(
        (menu_right - trigger_right).abs() < 12.0,
        "menu right edge {menu_right} should line up with the trigger {trigger_right}"
    );
    tap(&mut rt, "option3");
    assert!(!rt.state.resolution_open);
}
