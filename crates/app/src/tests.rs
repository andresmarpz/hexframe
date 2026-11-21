use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::settings::{Quality, Resolution, Settings};
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{Env, Key, KeyEvent, Mods, Runtime};

use crate::ui::fake_player::{self, Call};
use crate::ui::{Detail, Source, Ui, parse_clip_secs};
use crate::view;

mod clips_screen;
mod coach;
mod delete_dialog;
mod death_review;
mod dock;
mod instant_pages;
mod keybinds;
mod match_flow;
mod nav;
mod nav_rail;
mod nav_recent;
mod nav_status;
mod panes;
mod review_flow;
mod search_clear;
mod selection;
mod settings_sheet;
mod storage_limit;
mod update;
mod update_banner;

fn runtime_with(ui: Ui) -> Runtime<Ui> {
    runtime_sized(ui, (1280.0, 720.0))
}

fn runtime_sized(ui: Ui, size: (f32, f32)) -> Runtime<Ui> {
    let mut rt = Runtime::new(ui, view::root, size, Env::new(true, 1.0));
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
    if rt.state.settings_open && key != "nav-settings" && key != "dock-settings" {
        let target = key.to_owned();
        rt.update(move |_, tx| tx.scroll_into_view(target));
        for _ in 0..40 {
            rt.frame(1.0 / 60.0);
        }
    }
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
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
fn save_clip_button_sends_the_command_while_recording() {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| ui.status = Ui::preview().status);
    rt.frame(1.0 / 60.0);
    tap(&mut rt, "save-clip");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::SaveClipNow));
}

#[test]
fn clicking_a_game_row_opens_its_review_in_place() {
    let (mut rt, _rx) = live_runtime();
    tap(&mut rt, "nav-games");
    settle(&mut rt);
    let id = rt.state.recordings[1].meta.id.clone();
    tap(&mut rt, &id);
    assert_eq!(rt.state.detail(), Detail::Game);
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.entry.meta.id.clone()),
        Some(id.clone())
    );
    let other = rt.state.recordings[2].meta.id.clone();
    tap(&mut rt, &other);
    assert_eq!(rt.state.selected_id(), Some(other.as_str()));
    assert!(rt.find(&id).is_some(), "the list stays while reviewing");
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
fn cursor_switch_flips_the_setting_and_applies_it() {
    let (mut rt, rx) = live_runtime();
    tap(&mut rt, "nav-settings");
    let before = rt.state.settings.capture_cursor;
    tap(&mut rt, "capture-cursor-switch");
    assert_eq!(rt.state.settings.capture_cursor, !before);
    let Ok(EngineCmd::ApplySettings(sent)) = rx.try_recv() else {
        panic!("no ApplySettings sent");
    };
    assert_eq!(sent.capture_cursor, !before);
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
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
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
fn tapping_a_marker_seeks_five_seconds_before_the_event() {
    let mut rt = review_runtime();
    tap(&mut rt, "marker-9");
    assert_seeks_to(3.2);
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
    tap(&mut rt, "event-6");
    assert_seeks_to(0.5);
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
    tap(&mut rt, "tab-clips");
    tap(&mut rt, "clip-1");
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.source),
        Some(Source::Clip(1))
    );
    assert!(matches!(fake_player::calls()[0], Call::Open(_)));
    rt.frame(1.0);
    tap(&mut rt, "review-game");
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.source),
        Some(Source::Game)
    );
}

#[test]
fn closing_the_game_drops_the_player() {
    let mut rt = key_review_runtime();
    press(&mut rt, Key::Escape);
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

fn press(rt: &mut Runtime<Ui>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    rt.frame(1.0 / 60.0);
}

fn press_char(rt: &mut Runtime<Ui>, c: &str) {
    press(rt, Key::Text(c.into()));
}

fn key_review_runtime() -> Runtime<Ui> {
    let mut rt = review_runtime();
    rt.on_key = Some(Ui::on_key);
    rt
}

fn in_theater(rt: &Runtime<Ui>) -> bool {
    rt.state.in_theater()
}

#[test]
fn f_toggles_theater_and_escape_leaves_it() {
    let mut rt = key_review_runtime();
    assert!(!in_theater(&rt));
    press_char(&mut rt, "f");
    assert!(in_theater(&rt));
    assert!(rt.state.window_full_screen);
    press_char(&mut rt, "F");
    assert!(!in_theater(&rt));
    press_char(&mut rt, "f");
    press(&mut rt, Key::Escape);
    assert!(!in_theater(&rt));
    assert!(!rt.state.window_full_screen);
    assert_eq!(rt.state.detail(), Detail::Game);
}

#[test]
fn escape_outside_theater_closes_the_game() {
    let mut rt = key_review_runtime();
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.detail(), Detail::Pick);
    assert!(rt.state.review.is_none());
}

#[test]
fn theater_shows_the_timeline_and_hides_the_navigation() {
    let mut rt = key_review_runtime();
    assert!(rt.find("nav-settings").is_some());
    press_char(&mut rt, "f");
    assert!(rt.find("timeline").is_some());
    assert!(rt.find("marker-9").is_some());
    assert!(rt.find("theater-exit").is_some());
    assert!(rt.find("nav-settings").is_none());
    assert!(rt.find("review-header").is_none());
    assert!(rt.find("event-5").is_none());
    assert!(rt.find("theater-top").is_some());
}

#[test]
fn the_theater_button_and_exit_button_switch_modes() {
    let mut rt = review_runtime();
    tap(&mut rt, "review-theater");
    assert!(in_theater(&rt));
    tap(&mut rt, "theater-exit");
    assert!(!in_theater(&rt));
}

#[test]
fn double_pressing_the_video_toggles_theater() {
    let mut rt = review_runtime();
    let (x, y) = rt.find("review-video").expect("video");
    for _ in 0..2 {
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
    }
    rt.frame(1.0 / 60.0);
    assert!(in_theater(&rt));
}

#[test]
fn a_marker_seeks_the_same_way_in_theater() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    fake_player::clear_calls();
    tap(&mut rt, "marker-9");
    assert_seeks_to(3.2);
}

#[test]
fn space_toggles_playback() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, " ");
    assert_eq!(fake_player::calls(), vec![Call::Play]);
}

#[test]
fn arrow_keys_seek_five_seconds() {
    let mut rt = key_review_runtime();
    fake_player::set_time(6.0);
    press(&mut rt, Key::Right);
    assert_seeks_to(10.0);
    fake_player::clear_calls();
    press(&mut rt, Key::Left);
    assert_seeks_to(1.0);
    fake_player::set_time(0.0);
}

#[test]
fn review_keys_do_nothing_without_a_game() {
    let mut rt = runtime();
    rt.on_key = Some(Ui::on_key);
    press_char(&mut rt, "f");
    assert_eq!(rt.state.detail(), Detail::Live);
    assert!(!in_theater(&rt));
}

fn size_estimate_text(rt: &Runtime<Ui>) -> String {
    rt.accessibility_tree()
        .nodes
        .iter()
        .flat_map(|(_, node)| [node.label(), node.value()])
        .flatten()
        .find(|text| text.contains("MB per minute"))
        .map(str::to_owned)
        .expect("size estimate text")
}

#[test]
fn changing_quality_updates_the_size_estimate() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    let before = size_estimate_text(&rt);
    let (x, y) = rt.find("quality").unwrap();
    rt.pointer_down(x + 60.0, y);
    rt.pointer_up(x + 60.0, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.settings.quality, Quality::Max);
    assert_ne!(size_estimate_text(&rt), before);
}

fn rect(rt: &Runtime<Ui>, key: &str) -> [f32; 4] {
    rt.presented(key)
        .unwrap_or_else(|| panic!("no presented {key}"))
        .0
}

fn refresh(rt: &mut Runtime<Ui>) {
    rt.update(|_, _| {});
    rt.frame(1.0 / 60.0);
}

fn play_button_rects(rt: &mut Runtime<Ui>) -> ([f32; 4], [f32; 4]) {
    fake_player::set_playing(false);
    refresh(rt);
    let paused = rect(rt, "review-play");
    fake_player::set_playing(true);
    refresh(rt);
    let playing = rect(rt, "review-play");
    fake_player::set_playing(false);
    (paused, playing)
}

#[test]
fn the_play_button_keeps_its_size_while_playing_and_paused() {
    let mut rt = review_runtime();
    let (paused, playing) = play_button_rects(&mut rt);
    assert_eq!(paused, playing);
    assert_eq!(paused[2], paused[3], "square");
}

#[test]
fn the_theater_play_button_keeps_its_size_while_playing_and_paused() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    let (paused, playing) = play_button_rects(&mut rt);
    assert_eq!(paused, playing);
    assert_eq!(paused[2], paused[3], "square");
}

fn clock_width_at(rt: &mut Runtime<Ui>, secs: f64) -> f32 {
    fake_player::set_time(secs);
    refresh(rt);
    rect(rt, "review-clock")[2]
}

#[test]
fn the_time_readout_keeps_its_width_as_the_digits_change() {
    let mut rt = review_runtime();
    let zero = clock_width_at(&mut rt, 0.0);
    assert_eq!(zero, clock_width_at(&mut rt, 1.0));
    assert_eq!(zero, clock_width_at(&mut rt, 7.0));
    assert_eq!(zero, clock_width_at(&mut rt, 71.0));
    fake_player::set_time(0.0);
}

#[test]
fn the_theater_time_readout_keeps_its_width_as_the_digits_change() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    let zero = clock_width_at(&mut rt, 0.0);
    assert_eq!(zero, clock_width_at(&mut rt, 1.0));
    assert_eq!(zero, clock_width_at(&mut rt, 8.0));
    assert_eq!(zero, clock_width_at(&mut rt, 71.0));
    fake_player::set_time(0.0);
}

fn hover(rt: &mut Runtime<Ui>, key: &str) {
    rt.frame(0.5);
    rt.frame(0.5);
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_move(x, y);
    rt.frame(0.0);
}

fn hover_away(rt: &mut Runtime<Ui>) {
    rt.pointer_move(2.0, 2.0);
    rt.frame(0.0);
}

fn card_text(rt: &Runtime<Ui>) -> String {
    rt.accessibility_tree()
        .nodes
        .iter()
        .flat_map(|(_, node)| [node.label(), node.value()])
        .flatten()
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn hovering_a_marker_shows_the_event_card_after_a_delay_and_leaving_hides_it() {
    let mut rt = review_runtime();
    assert!(rt.find("event-card").is_none());
    hover(&mut rt, "marker-5");
    rt.frame(0.1);
    assert!(rt.find("event-card").is_none(), "not before 150 ms");
    rt.frame(0.1);
    assert!(rt.find("event-card").is_some());
    let text = card_text(&rt);
    for expected in ["Lee Sin", "killed", "You"] {
        assert!(text.contains(expected), "{expected} missing in {text}");
    }
    let card = rect(&rt, "event-card");
    let marker = rect(&rt, "marker-5");
    assert!(
        card[1] + card[3] <= marker[1] + 1.0,
        "card sits above its marker"
    );
    hover_away(&mut rt);
    rt.frame(0.5);
    assert!(rt.find("event-card").is_none());
}

#[test]
fn hovering_a_turret_marker_decodes_the_structure() {
    let mut rt = review_runtime();
    hover(&mut rt, "marker-4");
    rt.frame(0.2);
    assert!(card_text(&rt).contains("Red mid"), "{}", card_text(&rt));
}

#[test]
fn hovering_an_event_row_shows_the_card_beside_the_panel() {
    let mut rt = review_runtime();
    hover(&mut rt, "event-8");
    rt.frame(0.2);
    assert!(rt.find("event-card").is_some());
    let card = rect(&rt, "event-card");
    let row = rect(&rt, "event-8");
    assert!(
        card[0] + card[2] <= row[0] + 1.0,
        "card sits left of the row"
    );
    hover_away(&mut rt);
    rt.frame(0.5);
    assert!(rt.find("event-card").is_none());
}

#[test]
fn a_passing_pointer_shows_no_card() {
    let mut rt = review_runtime();
    hover(&mut rt, "marker-5");
    rt.frame(0.05);
    hover_away(&mut rt);
    rt.frame(0.5);
    assert!(rt.find("event-card").is_none());
}

#[test]
fn the_event_card_also_shows_in_theater() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    hover(&mut rt, "marker-8");
    rt.frame(0.2);
    assert!(rt.find("event-card").is_some());
}

fn nav_rect_of(rt: &Runtime<Ui>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("no element {key}"))
        .rect
}

fn settle(rt: &mut Runtime<Ui>) {
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn the_games_list_is_300_points_wide_and_the_settings_row_opens_the_sheet() {
    let mut rt = runtime();
    let list = nav_rect_of(&rt, "sidebar");
    assert!((list[2] - 300.0).abs() < 24.0, "width {}", list[2]);
    let nav = nav_rect_of(&rt, "nav");
    assert!((nav[2] - 220.0).abs() < 1.0, "width {}", nav[2]);
    assert!(rt.find("settings-sheet").is_none());
    tap(&mut rt, "nav-settings");
    assert!(rt.state.settings_open);
    assert!(rt.find("settings-sheet").is_some());
    press(&mut rt, Key::Escape);
    assert!(!rt.state.settings_open);
}

#[test]
fn the_settings_sheet_scrolls_while_the_games_list_stays() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    assert!(rt.find("settings-scroll").is_some());
    assert!(rt.find("filter-champion").is_some());
}

fn visible_texts(rt: &Runtime<Ui>) -> Vec<String> {
    rt.accessibility_tree()
        .nodes
        .iter()
        .flat_map(|(_, node)| [node.label(), node.value()])
        .flatten()
        .map(str::to_owned)
        .collect()
}

fn open_resolution_menu(rt: &mut Runtime<Ui>) {
    tap(rt, "nav-settings");
    tap(rt, "resolution-trigger");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn picking_a_height_from_the_menu_stores_that_height() {
    let mut rt = runtime();
    open_resolution_menu(&mut rt);
    tap(&mut rt, "option3");
    assert_eq!(rt.state.settings.resolution, Resolution::Height(1200));
    assert!(!rt.state.resolution_open);
    open_resolution_menu_again(&mut rt);
    tap(&mut rt, "option0");
    assert_eq!(rt.state.settings.resolution, Resolution::Native);
}

fn open_resolution_menu_again(rt: &mut Runtime<Ui>) {
    tap(rt, "resolution-trigger");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn the_menu_lists_native_then_the_ladder_without_a_detected_display() {
    let mut rt = runtime();
    open_resolution_menu(&mut rt);
    let texts = visible_texts(&rt);
    for label in ["Native", "2160p", "1440p", "1200p", "720p"] {
        assert!(texts.iter().any(|t| t == label), "missing {label}");
    }
}

#[test]
fn a_saved_height_outside_the_ladder_stays_selectable() {
    let mut rt = runtime();
    rt.update(|ui, _| ui.settings.resolution = Resolution::Height(800));
    rt.frame(1.0 / 60.0);
    open_resolution_menu(&mut rt);
    assert!(visible_texts(&rt).iter().any(|t| t == "800p"));
}
