use rune::{Key, Runtime};

use super::{press, press_char, rect, runtime_sized};
use crate::ui::fake_player::sim::{self, Media};
use crate::ui::{Ui, death_entry, review_entry};

fn opened(entry: greyscreen_engine::RecordingEntry) -> Runtime<Ui> {
    sim::simulate(Media::new(60.0));
    let mut ui = Ui::preview();
    ui.player_factory = sim::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.on_key = Some(Ui::on_key);
    rt.update(move |ui, _| ui.open_review_entry(entry));
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    rt
}

fn playing(rt: &Runtime<Ui>) -> bool {
    rt.state.review.as_ref().expect("review").is_playing()
}

fn click_at(rt: &mut Runtime<Ui>, x: f32, y: f32) {
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

fn click(rt: &mut Runtime<Ui>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    click_at(rt, x, y);
}

fn double_click(rt: &mut Runtime<Ui>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    for _ in 0..2 {
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
    }
    rt.frame(1.0 / 60.0);
}

fn middle_of(rt: &Runtime<Ui>, key: &str) -> (f32, f32) {
    let [x, y, w, h] = rect(rt, key);
    (x + w / 2.0, y + h / 2.0)
}

#[test]
fn clicking_the_video_pauses_then_plays() {
    let mut rt = opened(review_entry());
    let start = playing(&rt);
    click(&mut rt, "review-video");
    assert_eq!(playing(&rt), !start);
    click(&mut rt, "review-video");
    assert_eq!(playing(&rt), start);
}

#[test]
fn double_clicking_the_video_enters_theater_and_keeps_the_playback() {
    for flip_first in [false, true] {
        let mut rt = opened(review_entry());
        if flip_first {
            click(&mut rt, "review-video");
        }
        let start = playing(&rt);
        double_click(&mut rt, "review-video");
        assert!(rt.state.in_theater());
        assert_eq!(playing(&rt), start);
        double_click(&mut rt, "review-video");
        assert!(!rt.state.in_theater());
        assert_eq!(playing(&rt), start);
    }
}

#[test]
fn clicking_the_controls_below_the_video_leaves_the_playback_alone() {
    let mut rt = opened(review_entry());
    let start = playing(&rt);
    let (x, y) = middle_of(&rt, "review-controls");
    click_at(&mut rt, x, y);
    assert_eq!(playing(&rt), start);
}

#[test]
fn clicking_the_video_in_theater_toggles_the_playback() {
    let mut rt = opened(review_entry());
    press_char(&mut rt, "f");
    let start = playing(&rt);
    click(&mut rt, "review-video");
    assert_eq!(playing(&rt), !start);
    assert!(rt.state.in_theater());
}

#[test]
fn clicking_a_theater_bar_or_the_drawer_leaves_the_playback_alone() {
    let mut rt = opened(review_entry());
    press_char(&mut rt, "f");
    click(&mut rt, "theater-events");
    let start = playing(&rt);
    for key in ["theater-top", "theater-bar", "theater-drawer"] {
        let [x, y, _, _] = rect(&rt, key);
        click_at(&mut rt, x + 2.0, y + 2.0);
        assert_eq!(playing(&rt), start, "a click on {key} reached the video");
    }
}

#[test]
fn clicking_the_video_while_a_menu_is_open_only_closes_the_menu() {
    let mut rt = opened(review_entry());
    let start = playing(&rt);
    click(&mut rt, "review-speed");
    assert!(rt.state.review.as_ref().is_some_and(|r| r.speed_menu));
    click(&mut rt, "review-video");
    assert!(!rt.state.review.as_ref().is_some_and(|r| r.speed_menu));
    assert_eq!(playing(&rt), start);
}

#[test]
fn space_on_the_focused_video_toggles_once() {
    let mut rt = opened(review_entry());
    let start = playing(&rt);
    rt.update(|_, tx| tx.focus("review-video", true));
    rt.frame(1.0 / 60.0);
    press(&mut rt, Key::Text(" ".into()));
    assert_eq!(playing(&rt), !start);
}

#[test]
fn clicking_the_video_in_a_death_review_toggles_the_playback() {
    let mut rt = opened(death_entry());
    press_char(&mut rt, "d");
    assert!(rt.state.review.as_ref().is_some_and(|r| r.death_active()));
    let before = playing(&rt);
    click(&mut rt, "review-video");
    assert_ne!(playing(&rt), before);
    click(&mut rt, "review-video");
    assert_eq!(playing(&rt), before);
    double_click(&mut rt, "review-video");
    assert_eq!(playing(&rt), before);
}
