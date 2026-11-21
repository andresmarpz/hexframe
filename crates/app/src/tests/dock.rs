use greyscreen_core::settings::NavStyle;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::Runtime;

use super::{live_runtime, nav_rect_of, runtime_sized, runtime_with, settle, tap, visible_texts};
use crate::ui::fake_player::{self, Call};
use crate::ui::{Screen, Ui};

fn docked() -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.settings.nav_style = NavStyle::Dock;
    ui.status = EngineStatus::Idle;
    let mut rt = runtime_sized(ui, (1180.0, 680.0));
    settle(&mut rt);
    rt
}

#[test]
fn the_dock_floats_at_the_bottom_center_and_replaces_the_nav_sidebar() {
    let rt = docked();
    let dock = nav_rect_of(&rt, "dock");
    assert!((dock[3] - 44.0).abs() < 1.0, "height {}", dock[3]);
    let centre = dock[0] + dock[2] / 2.0;
    assert!((centre - 590.0).abs() < 2.0, "centre {centre}");
    assert!(dock[1] + dock[3] <= 680.0 - 15.0, "bottom {}", dock[1]);
    assert!(rt.find("nav").is_none());
    assert!(rt.find("nav-home").is_none());
}

#[test]
fn the_default_style_keeps_the_nav_sidebar_and_has_no_dock() {
    let (rt, _rx) = live_runtime();
    assert!(rt.find("nav").is_some());
    assert!(rt.find("dock").is_none());
}

#[test]
fn the_dock_tabs_switch_screens() {
    let mut rt = docked();
    tap(&mut rt, "dock-games");
    assert_eq!(rt.state.screen, Screen::Games);
    assert!(rt.find("filter-champion").is_some());
    tap(&mut rt, "dock-clips");
    assert_eq!(rt.state.screen, Screen::Clips);
    assert!(rt.find("clips-grid").is_some());
    tap(&mut rt, "dock-home");
    assert_eq!(rt.state.screen, Screen::Home);
}

#[test]
fn the_dock_stays_on_screen_in_every_screen() {
    let mut rt = docked();
    for tab in ["dock-games", "dock-clips", "dock-home"] {
        tap(&mut rt, tab);
        assert!(rt.find("dock").is_some(), "dock missing after {tab}");
    }
}

#[test]
fn the_gear_opens_and_closes_the_settings_sheet() {
    let mut rt = docked();
    tap(&mut rt, "dock-settings");
    assert!(rt.state.settings_open);
    assert!(rt.find("settings-sheet").is_some());
    tap(&mut rt, "dock-settings");
    assert!(!rt.state.settings_open);
}

#[test]
fn the_rec_switch_toggles_record_games() {
    let mut rt = docked();
    assert!(rt.state.settings.auto_record);
    tap(&mut rt, "record-games");
    assert!(!rt.state.settings.auto_record);
}

fn reviewing() -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.settings.nav_style = NavStyle::Dock;
    ui.status = EngineStatus::Idle;
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    settle(&mut rt);
    fake_player::clear_calls();
    fake_player::set_playing(false);
    rt
}

fn play_for_a_while(rt: &mut Runtime<Ui>) {
    fake_player::set_playing(true);
    for i in 0..200 {
        let clock = 10.0 + f64::from(i) * 0.05;
        rt.update(move |ui, tx| {
            ui.poll(clock, tx);
        });
        rt.frame(1.0 / 60.0);
    }
    settle(rt);
}

#[test]
fn the_dock_grows_while_recording_and_shows_the_clock_and_champion() {
    let mut ui = Ui::preview();
    ui.settings.nav_style = NavStyle::Dock;
    let mut rt = runtime_with(ui);
    settle(&mut rt);
    assert!(rt.find("dock-live").is_some());
    assert!(rt.find("dock-clip").is_some());
    assert!(visible_texts(&rt).iter().any(|t| t.contains("12:34")));
    let wide = nav_rect_of(&rt, "dock-record")[2];
    rt.update(|ui, _| ui.status = EngineStatus::Idle);
    settle(&mut rt);
    assert!(rt.find("dock-live").is_none());
    assert!(rt.find("record-games").is_some());
    assert!(nav_rect_of(&rt, "dock-record")[2] < wide - 100.0);
}

#[test]
fn the_clip_button_saves_a_clip_while_recording() {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| {
        ui.settings.nav_style = NavStyle::Dock;
        ui.status = EngineStatus::Recording {
            since: 0,
            game: None,
        };
    });
    settle(&mut rt);
    tap(&mut rt, "dock-clip");
    assert!(matches!(rx.try_iter().last(), Some(EngineCmd::SaveClipNow)));
}

#[test]
fn a_test_recording_reads_test_and_clicking_the_live_segment_goes_home() {
    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| {
        ui.settings.nav_style = NavStyle::Dock;
        ui.status = EngineStatus::Recording {
            since: 0,
            game: None,
        };
        ui.go(Screen::Clips);
    });
    settle(&mut rt);
    let said_test = rt.accessibility_tree().nodes.iter().any(|(_, node)| {
        node.label()
            .is_some_and(|l| l.starts_with("Recording Test,"))
    });
    assert!(said_test);
    tap(&mut rt, "dock-live");
    assert_eq!(rt.state.screen, Screen::Home);
}

#[test]
fn the_dock_stays_while_a_game_is_open_and_paused() {
    let rt = reviewing();
    assert!(rt.find("dock").is_some());
    assert!(rt.find("dock-zone").is_some());
}

#[test]
fn the_review_keeps_clear_of_the_dock() {
    let rt = reviewing();
    let dock = nav_rect_of(&rt, "dock");
    let play = nav_rect_of(&rt, "review-play");
    assert!(
        play[1] + play[3] <= dock[1],
        "play {play:?} sits under dock {dock:?}"
    );
}

#[test]
fn the_dock_slides_away_while_the_video_plays_and_the_review_takes_the_room() {
    let mut rt = reviewing();
    let before = nav_rect_of(&rt, "review-play")[1];
    play_for_a_while(&mut rt);
    assert!(rt.find("dock").is_none(), "hidden while playing");
    assert!(rt.find("dock-zone").is_some());
    assert!(nav_rect_of(&rt, "review-play")[1] > before);
}

#[test]
fn the_dock_returns_on_pause() {
    let mut rt = reviewing();
    play_for_a_while(&mut rt);
    assert!(rt.find("dock").is_none());
    fake_player::set_playing(false);
    rt.update(|ui, tx| {
        ui.poll(40.0, tx);
    });
    settle(&mut rt);
    assert!(rt.find("dock").is_some());
}

#[test]
fn moving_near_the_bottom_wakes_the_dock_while_playing() {
    let mut rt = reviewing();
    play_for_a_while(&mut rt);
    assert!(rt.find("dock").is_none());
    rt.pointer_move(700.0, 660.0);
    rt.frame(1.0 / 60.0);
    settle(&mut rt);
    assert!(
        rt.find("dock").is_some(),
        "pointer near the bottom shows it"
    );
}

#[test]
fn the_hover_zone_does_not_block_the_player_controls() {
    let mut rt = reviewing();
    play_for_a_while(&mut rt);
    fake_player::clear_calls();
    tap(&mut rt, "review-play");
    assert_eq!(fake_player::calls(), vec![Call::Pause]);
}

#[test]
fn the_sidebar_style_has_no_hover_zone_and_never_hides() {
    let mut ui = Ui::preview();
    ui.status = EngineStatus::Idle;
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    settle(&mut rt);
    assert!(rt.find("dock-zone").is_none());
    assert!(rt.find("dock-reserve").is_none());
}
