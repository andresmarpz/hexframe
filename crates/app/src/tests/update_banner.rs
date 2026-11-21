#![allow(clippy::unwrap_used, clippy::expect_used)]

use greyscreen_core::settings::NavStyle;
use greyscreen_core::update::{UpdateInfo, UpdateState};
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::Runtime;

use super::{live_runtime, settle, tap, visible_texts};
use crate::ui::{Screen, Ui};

fn info(tag: &str) -> UpdateInfo {
    UpdateInfo {
        tag: tag.into(),
        prerelease: false,
        size: 2,
        notes_url: greyscreen_core::update::notes_url(tag),
    }
}

fn ready(rt: &mut Runtime<Ui>, tag: &str) {
    let info = info(tag);
    rt.update(move |ui, _| ui.apply_update(UpdateState::Ready(info)));
    settle(rt);
}

fn shown(rt: &Runtime<Ui>) -> bool {
    rt.find("update-banner").is_some()
}

fn press(rt: &mut Runtime<Ui>, label: &str) {
    let node = rt
        .accessibility_tree()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label))
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| panic!("no button {label}"));
    let b = node.bounds().expect("bounds");
    let (x, y) = (((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

#[test]
fn a_ready_update_shows_the_banner_with_its_version() {
    let (mut rt, _rx) = live_runtime();
    assert!(!shown(&rt));
    ready(&mut rt, "v0.2.0");
    assert!(shown(&rt));
    assert!(visible_texts(&rt).contains(&"Greyscreen 0.2.0 is ready".to_owned()));
}

#[test]
fn only_a_ready_update_shows_it() {
    let (mut rt, _rx) = live_runtime();
    for state in [
        UpdateState::Available(info("v0.2.0")),
        UpdateState::Downloading(info("v0.2.0")),
        UpdateState::AvailableManual(info("v0.2.0")),
        UpdateState::Installing(info("v0.2.0")),
    ] {
        rt.update(move |ui, _| ui.apply_update(state));
        settle(&mut rt);
        assert!(!shown(&rt));
    }
}

#[test]
fn it_shows_on_every_screen_in_both_nav_styles() {
    for style in [NavStyle::Sidebar, NavStyle::Dock] {
        let (mut rt, _rx) = live_runtime();
        rt.update(move |ui, _| ui.settings.nav_style = style);
        ready(&mut rt, "v0.2.0");
        for screen in [Screen::Home, Screen::Games, Screen::Clips] {
            rt.update(move |ui, _| ui.go(screen));
            settle(&mut rt);
            assert!(shown(&rt), "{style:?} {screen:?}");
        }
    }
}

#[test]
fn restart_sends_the_install_command() {
    let (mut rt, rx) = live_runtime();
    ready(&mut rt, "v0.2.0");
    press(&mut rt, "Restart to update");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::InstallUpdate));
}

#[test]
fn later_hides_it_until_a_newer_version() {
    let (mut rt, rx) = live_runtime();
    ready(&mut rt, "v0.2.0");
    press(&mut rt, "Later");
    settle(&mut rt);
    assert!(!shown(&rt));
    assert!(rx.try_recv().is_err());
    ready(&mut rt, "v0.2.0");
    assert!(!shown(&rt));
    ready(&mut rt, "v0.3.0");
    assert!(shown(&rt));
}

#[test]
fn it_hides_while_a_game_is_recording() {
    let (mut rt, _rx) = live_runtime();
    ready(&mut rt, "v0.2.0");
    rt.update(|ui, _| ui.status = Ui::preview().status);
    settle(&mut rt);
    assert!(matches!(rt.state.status, EngineStatus::Recording { .. }));
    assert!(!shown(&rt));
    rt.update(|ui, _| ui.status = EngineStatus::Idle);
    settle(&mut rt);
    assert!(shown(&rt));
}

#[test]
fn it_hides_in_theater_mode() {
    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    ready(&mut rt, "v0.2.0");
    assert!(shown(&rt));
    rt.update(|ui, _| {
        if let Some(r) = ui.review.as_mut() {
            r.theater = true;
        }
    });
    settle(&mut rt);
    assert!(!shown(&rt));
}

#[test]
fn the_banner_does_not_cover_a_nav_button() {
    let (mut rt, _rx) = live_runtime();
    ready(&mut rt, "v0.2.0");
    tap(&mut rt, "nav-settings");
    assert!(rt.state.settings_open);
}
