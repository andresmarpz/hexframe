use rune::{Env, Runtime};

use crate::ui::{Screen, Ui, parse_clip_secs};
use crate::view;

fn runtime() -> Runtime<Ui> {
    let mut rt = Runtime::new(
        Ui::preview(),
        view::root,
        (960.0, 640.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    rt
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
fn toggling_a_switch_changes_the_setting() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    let before = rt.state.settings.audio;
    tap(&mut rt, "audio-switch");
    assert_eq!(rt.state.settings.audio, !before);
    tap(&mut rt, "audio-switch");
    assert_eq!(rt.state.settings.audio, before);
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
