use greyscreen_engine::EngineStatus;
use rune::Runtime;

use super::{nav_rect_of, runtime, settle, tap, visible_texts};
use crate::ui::{Screen, Ui};

fn recording_runtime(screen: Screen) -> Runtime<Ui> {
    let mut rt = runtime();
    rt.update(move |ui, _| ui.screen = screen);
    settle(&mut rt);
    rt
}

#[test]
fn the_sidebar_is_about_220_points_wide_and_on_the_left() {
    let rt = runtime();
    let nav = nav_rect_of(&rt, "nav");
    assert_eq!(nav[0], 0.0);
    assert!((nav[2] - 220.0).abs() < 1.0, "width {}", nav[2]);
    assert!(rt.find("nav-home").is_some());
    assert!(rt.find("nav-recordings").is_some());
    assert!(rt.find("nav-settings").is_some());
}

#[test]
fn the_sidebar_items_switch_screens() {
    let mut rt = runtime();
    assert_eq!(rt.state.screen, Screen::Home);
    tap(&mut rt, "nav-recordings");
    assert_eq!(rt.state.screen, Screen::Recordings);
    tap(&mut rt, "nav-home");
    assert_eq!(rt.state.screen, Screen::Home);
}

#[test]
fn the_gear_opens_settings_and_has_a_tooltip_name() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    assert_eq!(rt.state.screen, Screen::Settings);
    assert!(visible_texts(&rt).iter().any(|t| t == "Settings"));
}

#[test]
fn recordings_stays_selected_while_reviewing_a_game() {
    let mut rt = runtime();
    let entry = rt.state.recordings[0].clone();
    rt.update(move |ui, _| ui.open_review_entry(entry));
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.screen, Screen::Review);
    assert!(rt.find("nav-recordings").is_some());
}

#[test]
fn the_indicator_shows_the_elapsed_time_on_recordings_while_recording() {
    let rt = recording_runtime(Screen::Recordings);
    assert!(rt.find("nav-recording").is_some());
    assert!(visible_texts(&rt).iter().any(|t| t.contains("12:34")));
}

#[test]
fn the_indicator_shows_on_settings_and_review_too() {
    let rt = recording_runtime(Screen::Settings);
    assert!(rt.find("nav-recording").is_some());
    let mut rt = runtime();
    let entry = rt.state.recordings[0].clone();
    rt.update(move |ui, _| ui.open_review_entry(entry));
    settle(&mut rt);
    assert!(rt.find("nav-recording").is_some());
}

#[test]
fn the_indicator_sits_in_the_footer_beside_the_gear() {
    let rt = recording_runtime(Screen::Recordings);
    let nav = nav_rect_of(&rt, "nav");
    let pill = nav_rect_of(&rt, "nav-recording");
    let gear = nav_rect_of(&rt, "nav-settings");
    let recordings = nav_rect_of(&rt, "nav-recordings");
    assert!(pill[0] + pill[2] <= gear[0]);
    assert!(gear[0] + gear[2] <= nav[0] + nav[2]);
    assert!(pill[1] > recordings[1] + recordings[3], "below the items");
    assert!(pill[1] + pill[3] <= nav[1] + nav[3]);
}

#[test]
fn the_indicator_is_hidden_on_home() {
    let rt = recording_runtime(Screen::Home);
    assert!(rt.find("nav-recording").is_none());
}

#[test]
fn the_indicator_is_hidden_when_not_recording() {
    let mut rt = recording_runtime(Screen::Recordings);
    rt.update(|ui, _| ui.status = EngineStatus::Idle);
    rt.frame(1.0 / 60.0);
    assert!(rt.find("nav-recording").is_none());
}

#[test]
fn the_indicator_follows_the_clock() {
    let mut rt = recording_runtime(Screen::Recordings);
    rt.update(|ui, _| ui.elapsed_secs = 61);
    rt.frame(1.0 / 60.0);
    assert!(visible_texts(&rt).iter().any(|t| t.contains("01:01")));
}

#[test]
fn clicking_the_indicator_goes_home() {
    let mut rt = recording_runtime(Screen::Settings);
    tap(&mut rt, "nav-recording");
    assert_eq!(rt.state.screen, Screen::Home);
    assert!(rt.find("nav-recording").is_none());
}
