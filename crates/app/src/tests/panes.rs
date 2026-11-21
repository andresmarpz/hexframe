use greyscreen_core::settings::NavStyle;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::Runtime;

use super::{live_runtime, nav_rect_of, review_runtime, runtime, runtime_with, settle};
use crate::ui::Ui;

fn drag(rt: &mut Runtime<Ui>, key: &str, dx: f32) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_move(x + dx / 2.0, y);
    rt.pointer_move(x + dx, y);
    rt.pointer_up(x + dx, y);
    settle(rt);
}

fn double_press(rt: &mut Runtime<Ui>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    for _ in 0..2 {
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
    }
    settle(rt);
}

fn width_of(rt: &Runtime<Ui>, key: &str) -> f32 {
    nav_rect_of(rt, key)[2]
}

fn idle_games(style: NavStyle) -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.status = EngineStatus::Idle;
    ui.settings.nav_style = style;
    ui.go(crate::ui::Screen::Games);
    runtime_with(ui)
}

#[test]
fn the_games_list_sits_inside_the_content_card_before_the_review() {
    for style in [NavStyle::Sidebar, NavStyle::Dock] {
        let rt = idle_games(style);
        let card = nav_rect_of(&rt, "main-pane");
        let list = nav_rect_of(&rt, "games-list");
        assert!(
            list[0] >= card[0] && list[0] + list[2] <= card[0] + card[2],
            "{style:?}"
        );
        assert!(list[1] >= card[1] && list[1] + list[3] <= card[1] + card[3] + 0.5);
        let body = nav_rect_of(&rt, "games-body");
        assert!(body[0] >= card[0] && body[0] + body[2] <= card[0] + card[2] + 0.5);
    }
}

#[test]
fn the_list_width_starts_at_the_default_and_the_review_takes_the_rest() {
    let rt = review_runtime();
    assert!((width_of(&rt, "games-list") - 300.0).abs() < 1.0);
    let list = nav_rect_of(&rt, "games-list");
    let card = nav_rect_of(&rt, "main-pane");
    let review = nav_rect_of(&rt, "review-player");
    assert!(review[0] > list[0] + list[2]);
    assert!(review[0] + review[2] <= card[0] + card[2] + 0.5);
}

#[test]
fn dragging_the_list_divider_changes_the_width_and_clamps_it() {
    let mut rt = runtime();
    drag(&mut rt, "list-grip", 60.0);
    assert_eq!(rt.state.settings.list_width, 360);
    assert!((width_of(&rt, "games-list") - 360.0).abs() < 1.0);
    drag(&mut rt, "list-grip", 500.0);
    assert_eq!(rt.state.settings.list_width, 440);
    drag(&mut rt, "list-grip", -900.0);
    assert_eq!(rt.state.settings.list_width, 240);
    assert!((width_of(&rt, "games-list") - 240.0).abs() < 1.0);
}

#[test]
fn dragging_the_nav_edge_changes_the_width_and_clamps_it() {
    let mut rt = runtime();
    drag(&mut rt, "nav-grip", 40.0);
    assert_eq!(rt.state.settings.nav_width, 260);
    assert!((width_of(&rt, "nav") - 260.0).abs() < 1.0);
    drag(&mut rt, "nav-grip", 500.0);
    assert_eq!(rt.state.settings.nav_width, 320);
    drag(&mut rt, "nav-grip", -900.0);
    assert_eq!(rt.state.settings.nav_width, 180);
}

#[test]
fn a_finished_drag_saves_the_settings_once_and_a_drag_in_flight_does_not() {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| ui.go(crate::ui::Screen::Games));
    settle(&mut rt);
    let before = rx.try_iter().count();
    let (x, y) = rt.find("list-grip").expect("grip");
    rt.pointer_down(x, y);
    rt.pointer_move(x + 30.0, y);
    rt.pointer_move(x + 50.0, y);
    assert_eq!(rx.try_iter().count(), 0, "saved mid-drag after {before}");
    rt.pointer_up(x + 50.0, y);
    settle(&mut rt);
    let saved: Vec<_> = rx
        .try_iter()
        .filter_map(|cmd| match cmd {
            EngineCmd::ApplySettings(settings) => Some(settings.list_width),
            _ => None,
        })
        .collect();
    assert_eq!(saved, vec![350]);
}

#[test]
fn both_widths_survive_a_restart_through_the_settings_file() {
    let mut rt = runtime();
    drag(&mut rt, "nav-grip", 30.0);
    drag(&mut rt, "list-grip", -40.0);
    let path = std::env::temp_dir().join(format!("greyscreen-panes-{}.json", std::process::id()));
    rt.state.settings.save(&path).unwrap();
    let loaded = greyscreen_core::settings::Settings::load(&path);
    std::fs::remove_file(&path).unwrap();
    let mut ui = Ui::preview();
    ui.status = EngineStatus::Idle;
    ui.settings = loaded;
    ui.go(crate::ui::Screen::Games);
    let rt = runtime_with(ui);
    assert!((width_of(&rt, "nav") - 250.0).abs() < 1.0);
    assert!((width_of(&rt, "games-list") - 260.0).abs() < 1.0);
}

#[test]
fn double_pressing_a_handle_resets_its_default() {
    let mut rt = runtime();
    drag(&mut rt, "list-grip", 100.0);
    drag(&mut rt, "nav-grip", 70.0);
    double_press(&mut rt, "list-grip");
    assert_eq!(rt.state.settings.list_width, 300);
    assert_eq!(rt.state.settings.nav_width, 290);
    double_press(&mut rt, "nav-grip");
    assert_eq!(rt.state.settings.nav_width, 220);
}

#[test]
fn the_rail_has_no_resize_handle_and_stays_56_wide() {
    let mut ui = Ui::preview();
    ui.settings.sidebar_collapsed = true;
    ui.settings.nav_width = 300;
    ui.go(crate::ui::Screen::Games);
    let rt = runtime_with(ui);
    assert!(rt.find("nav-grip").is_none());
    assert!((width_of(&rt, "nav") - 56.0).abs() < 1.0);
}

#[test]
fn a_narrow_window_still_folds_the_nav_to_the_rail() {
    let mut ui = Ui::preview();
    ui.go(crate::ui::Screen::Games);
    let rt = super::runtime_sized(ui, (1000.0, 600.0));
    assert!(rt.find("nav-grip").is_none());
    assert!((width_of(&rt, "nav") - 56.0).abs() < 1.0);
    assert!(rt.find("list-grip").is_some());
}
