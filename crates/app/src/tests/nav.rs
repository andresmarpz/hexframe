use greyscreen_engine::EngineStatus;
use rune::{Key, KeyEvent, Mods, Runtime};

use super::keybinds::platform;
use super::{live_runtime, press, press_char, runtime, settle, tap};
use crate::ui::fake_player::{self, Call};
use crate::ui::{Detail, Screen, Ui};

fn chord(rt: &mut Runtime<Ui>, c: char) {
    rt.key(KeyEvent {
        key: Key::Command(c),
        mods: platform(),
    });
    settle(rt);
}

fn idle_games() -> Runtime<Ui> {
    let mut rt = runtime();
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| {
        ui.status = EngineStatus::Idle;
        ui.close_game();
    });
    settle(&mut rt);
    rt
}

#[test]
fn the_app_opens_on_home() {
    let (rt, _rx) = live_runtime();
    assert_eq!(rt.state.screen, Screen::Home);
    assert!(rt.find("nav-home").is_some());
    assert!(rt.find("nav-games").is_some());
    assert!(rt.find("filter-champion").is_none());
}

#[test]
fn the_nav_is_220_points_wide_and_counts_the_games() {
    let (rt, _rx) = live_runtime();
    let nav = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("nav"))
        .expect("nav");
    assert!((nav.rect[2] - 220.0).abs() < 1.0, "width {}", nav.rect[2]);
    let count = rt.state.recordings.len().to_string();
    let row_said = rt
        .accessibility_tree()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Games") && node.description().is_some())
        .and_then(|(_, node)| node.description().map(str::to_owned));
    assert_eq!(row_said, Some(count));
}

#[test]
fn the_nav_rows_switch_between_home_and_games() {
    let (mut rt, _rx) = live_runtime();
    tap(&mut rt, "nav-games");
    assert_eq!(rt.state.screen, Screen::Games);
    assert!(rt.find("filter-champion").is_some());
    tap(&mut rt, "nav-home");
    assert_eq!(rt.state.screen, Screen::Home);
    assert!(rt.find("filter-champion").is_none());
}

#[test]
fn the_nav_stays_on_screen_in_both_screens() {
    let (mut rt, _rx) = live_runtime();
    assert!(rt.find("nav-settings").is_some());
    tap(&mut rt, "nav-games");
    assert!(rt.find("nav-settings").is_some());
    tap(&mut rt, "nav-settings");
    assert!(rt.state.settings_open);
}

#[test]
fn the_platform_modifier_with_1_and_2_switches_screens() {
    let mut rt = idle_games();
    chord(&mut rt, '1');
    assert_eq!(rt.state.screen, Screen::Home);
    assert_eq!(rt.state.detail(), Detail::Welcome);
    chord(&mut rt, '2');
    assert_eq!(rt.state.screen, Screen::Games);
}

#[test]
fn games_with_nothing_open_shows_the_placeholder() {
    let rt = idle_games();
    assert_eq!(rt.state.detail(), Detail::Pick);
    assert!(rt.find("games-pick").is_some());
    assert!(rt.find("welcome").is_none());
}

#[test]
fn escape_in_games_goes_back_to_the_placeholder() {
    let mut rt = idle_games();
    let first = rt.state.recordings[rt.state.visible_games()[0]]
        .meta
        .id
        .clone();
    tap(&mut rt, &first);
    assert_eq!(rt.state.detail(), Detail::Game);
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.detail(), Detail::Pick);
    assert_eq!(rt.state.screen, Screen::Games);
    assert!(rt.find("games-pick").is_some());
}

#[test]
fn arrow_keys_only_move_through_games_on_the_games_screen() {
    let mut rt = idle_games();
    chord(&mut rt, '1');
    press(&mut rt, Key::Down);
    assert!(rt.state.review.is_none());
    chord(&mut rt, '2');
    press(&mut rt, Key::Down);
    assert!(rt.state.review.is_some());
}

#[test]
fn opening_a_clip_from_home_switches_to_games() {
    let mut rt = idle_games();
    chord(&mut rt, '1');
    let id = rt.state.recordings[0].meta.id.clone();
    rt.update(move |ui, _| ui.open_clip(id, 0));
    settle(&mut rt);
    assert_eq!(rt.state.screen, Screen::Games);
    assert_eq!(rt.state.detail(), Detail::Game);
}

#[test]
fn the_search_key_goes_to_games_and_focuses_the_search() {
    let mut rt = idle_games();
    chord(&mut rt, '1');
    chord(&mut rt, 'f');
    assert_eq!(rt.state.screen, Screen::Games);
    let elements = rt.elements();
    let search = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("filter-champion"))
        .expect("search")
        .rect;
    let focused = elements.iter().find(|e| e.focused).expect("focus").rect;
    assert!(focused[0] >= search[0] && focused[0] + focused[2] <= search[0] + search[2] + 0.5);
    assert!(focused[1] >= search[1] && focused[1] + focused[3] <= search[1] + search[3] + 0.5);
}

#[test]
fn the_games_list_toggle_key_does_nothing_on_home() {
    let mut rt = idle_games();
    chord(&mut rt, '1');
    rt.key(KeyEvent {
        key: Key::Command('b'),
        mods: Mods {
            shift: true,
            ..platform()
        },
    });
    settle(&mut rt);
    assert!(rt.state.sidebar.is_open());
}

#[test]
fn leaving_games_pauses_the_review_and_coming_back_keeps_it() {
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    ui.status = EngineStatus::Idle;
    let mut rt = super::runtime_with(ui);
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    settle(&mut rt);
    fake_player::set_playing(true);
    fake_player::clear_calls();
    tap(&mut rt, "nav-home");
    assert_eq!(fake_player::calls(), vec![Call::Pause]);
    tap(&mut rt, "nav-games");
    assert_eq!(rt.state.detail(), Detail::Game);
}

#[test]
fn navigation_keys_do_nothing_in_theater() {
    let mut rt = super::key_review_runtime();
    press_char(&mut rt, "f");
    assert!(rt.state.in_theater());
    chord(&mut rt, '1');
    assert_eq!(rt.state.screen, Screen::Games);
    assert!(rt.state.in_theater());
}

#[test]
fn a_live_recording_shows_its_feed_on_home() {
    let mut rt = runtime();
    rt.update(|ui, _| ui.go(Screen::Home));
    settle(&mut rt);
    assert_eq!(rt.state.detail(), Detail::Live);
    assert!(rt.find("feed-card").is_some());
}
