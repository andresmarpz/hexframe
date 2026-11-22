use greyscreen_core::settings::NavStyle;
use greyscreen_engine::EngineCmd;
use rune::{Key, KeyEvent, Mods};

use super::{live_runtime, nav_rect_of, runtime, settle, tap};
use crate::ui::library::format::recent_games;
use crate::ui::{Screen, Ui};

fn nav_width(rt: &rune::Runtime<Ui>) -> f32 {
    nav_rect_of(rt, "nav")[2]
}

fn chord(rt: &mut rune::Runtime<Ui>, key: char) {
    rt.key(KeyEvent {
        key: Key::Command(key),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    });
    settle(rt);
}

#[test]
fn the_button_folds_the_nav_to_a_56_point_rail_and_back() {
    let (mut rt, rx) = live_runtime();
    tap(&mut rt, "nav-collapse");
    settle(&mut rt);
    assert!(rt.state.settings.sidebar_collapsed);
    assert!((nav_width(&rt) - 56.0).abs() < 1.0, "{}", nav_width(&rt));
    assert!(rx.try_iter().any(|cmd| matches!(
        cmd,
        EngineCmd::ApplySettings(settings) if settings.sidebar_collapsed
    )));
    for key in [
        "nav-home",
        "nav-games",
        "nav-clips",
        "nav-settings",
        "nav-status",
    ] {
        assert!(rt.find(key).is_some(), "{key}");
    }
    tap(&mut rt, "nav-collapse");
    settle(&mut rt);
    assert!(!rt.state.settings.sidebar_collapsed);
    assert!((nav_width(&rt) - 220.0).abs() < 1.0);
}

#[test]
fn mod_backslash_folds_and_unfolds_the_nav() {
    let mut rt = runtime();
    rt.on_key = Some(Ui::on_key);
    for expected in [true, false] {
        chord(&mut rt, '\\');
        assert_eq!(rt.state.settings.sidebar_collapsed, expected);
    }
}

#[test]
fn mod_backslash_leaves_the_dock_alone() {
    let mut rt = runtime();
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| ui.settings.nav_style = NavStyle::Dock);
    settle(&mut rt);
    chord(&mut rt, '\\');
    assert!(!rt.state.settings.sidebar_collapsed);
}

#[test]
fn the_rail_navigates_and_opens_recent_games() {
    let mut rt = runtime();
    rt.update(|ui, _| {
        ui.close_game();
        ui.screen = Screen::Home;
        ui.settings.sidebar_collapsed = true;
    });
    settle(&mut rt);
    assert!(rt.find("nav-recent").is_some());
    tap(&mut rt, "nav-clips");
    settle(&mut rt);
    assert_eq!(rt.state.screen, Screen::Clips);
    assert!(rt.find("nav-recent").is_some());
    let id = recent_games(&rt.state.recordings, 5)[1].meta.id.clone();
    tap(&mut rt, &format!("nav-game-{id}"));
    settle(&mut rt);
    assert_eq!(rt.state.screen, Screen::Games);
    assert_eq!(rt.state.selected_id(), Some(id.as_str()));
    assert!(rt.find("nav-recent").is_none());
    tap(&mut rt, "nav-status");
    settle(&mut rt);
    assert_eq!(rt.state.screen, Screen::Home);
    assert!(rt.find("nav-recent").is_some());
}

#[test]
fn a_narrow_window_folds_the_nav_to_the_rail_without_changing_the_setting() {
    let (mut rt, _rx) = live_runtime();
    rt.set_viewport((1000.0, 600.0), 1.0);
    settle(&mut rt);
    assert!((nav_width(&rt) - 56.0).abs() < 1.0, "{}", nav_width(&rt));
    assert!(!rt.state.settings.sidebar_collapsed);
    rt.set_viewport((1400.0, 800.0), 1.0);
    settle(&mut rt);
    assert!((nav_width(&rt) - 220.0).abs() < 1.0, "{}", nav_width(&rt));
    assert!(!rt.state.settings.sidebar_collapsed);
}
