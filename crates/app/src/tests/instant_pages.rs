use std::collections::BTreeMap;

use greyscreen_core::settings::NavStyle;
use greyscreen_engine::EngineStatus;
use rune::Runtime;

use super::{runtime, settle, tap};
use crate::ui::{Screen, Ui};

fn browsing() -> Runtime<Ui> {
    let mut rt = runtime();
    rt.update(|ui, _| {
        ui.status = EngineStatus::Idle;
        ui.close_game();
    });
    settle(&mut rt);
    rt
}

#[derive(Clone, Copy)]
struct Look {
    rect: [f32; 4],
    opacity: f32,
    leaving: bool,
}

fn looks(rt: &Runtime<Ui>) -> BTreeMap<String, Look> {
    rt.elements()
        .into_iter()
        .filter(|e| e.loop_phase.is_none() && !e.path.contains("dock"))
        .map(|e| {
            let look = Look {
                rect: e.rect,
                opacity: e.opacity,
                leaving: e.exiting,
            };
            (e.path, look)
        })
        .collect()
}

fn visibly_animating(rt: &mut Runtime<Ui>) -> Vec<String> {
    let first = looks(rt);
    settle(rt);
    let last = looks(rt);
    let mut found = Vec::new();
    for (path, a) in &first {
        if a.leaving {
            if a.opacity > 0.01 {
                found.push(format!("{path} still visible while leaving"));
            }
            continue;
        }
        let Some(b) = last.get(path) else { continue };
        let moved = a.rect.iter().zip(b.rect).any(|(x, y)| (x - y).abs() > 0.5);
        if moved || (a.opacity - b.opacity).abs() > 0.01 {
            found.push(format!(
                "{path} op {:.2}->{:.2} rect {:?}->{:?}",
                a.opacity, b.opacity, a.rect, b.rect
            ));
        }
    }
    found
}

fn nth_game(rt: &Runtime<Ui>, n: usize) -> String {
    rt.state.recordings[rt.state.visible_games()[n]]
        .meta
        .id
        .clone()
}

fn animating(rt: &mut Runtime<Ui>) -> Vec<String> {
    visibly_animating(rt)
        .into_iter()
        .filter(|path| !path.contains("sidebar-list"))
        .collect()
}

fn walk(rt: &mut Runtime<Ui>) {
    tap(rt, "nav-games");
    assert_eq!(animating(rt), Vec::<String>::new(), "opening Games");
    let first = nth_game(rt, 0);
    tap(rt, &first);
    assert_eq!(animating(rt), Vec::<String>::new(), "opening a game");
    let second = nth_game(rt, 1);
    tap(rt, &second);
    assert_eq!(animating(rt), Vec::<String>::new(), "moving to the next game");
    tap(rt, "nav-clips");
    assert_eq!(animating(rt), Vec::<String>::new(), "opening Clips");
    tap(rt, "nav-home");
    assert_eq!(rt.state.screen, Screen::Home);
    assert_eq!(animating(rt), Vec::<String>::new(), "opening Home");
}

#[test]
fn moving_between_pages_and_games_lands_in_one_frame() {
    let mut rt = browsing();
    walk(&mut rt);
}

#[test]
fn the_dock_layout_also_lands_pages_in_one_frame() {
    let mut rt = browsing();
    rt.update(|ui, _| ui.settings.nav_style = NavStyle::Dock);
    settle(&mut rt);
    let first = nth_game(&rt, 0);
    rt.update(|ui, _| ui.go(Screen::Games));
    settle(&mut rt);
    tap(&mut rt, &first);
    assert_eq!(animating(&mut rt), Vec::<String>::new(), "opening a game");
    rt.update(|ui, _| ui.go(Screen::Home));
    rt.frame(1.0 / 60.0);
    assert_eq!(animating(&mut rt), Vec::<String>::new(), "opening Home");
}

#[test]
fn switching_settings_sections_shows_the_new_section_on_the_next_frame() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    settle(&mut rt);
    let (x, y) = rt.find("settings-nav-clips").expect("clips section");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert!(rt.find("clips-enabled").is_some());
    assert!(rt.find("quality-row").is_none());
    assert_eq!(visibly_animating(&mut rt), Vec::<String>::new());
}
