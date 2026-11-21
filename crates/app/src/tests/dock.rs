use greyscreen_core::settings::NavStyle;
use greyscreen_engine::EngineStatus;
use rune::Runtime;

use super::{live_runtime, nav_rect_of, runtime_with, settle, tap};
use crate::ui::{Screen, Ui};

fn docked() -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.settings.nav_style = NavStyle::Dock;
    ui.status = EngineStatus::Idle;
    let mut rt = runtime_with(ui);
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
