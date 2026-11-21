#![allow(clippy::expect_used)]

use rune::Runtime;

use super::{rect, refresh, review_runtime, tap};
use crate::ui::fake_player::{self, Call};
use crate::ui::{Ui, review_entry};

fn volume(rt: &Runtime<Ui>) -> f32 {
    rt.state.review.as_ref().expect("review").volume
}

fn muted(rt: &Runtime<Ui>) -> bool {
    rt.state.review.as_ref().expect("review").muted
}

fn open_popover(rt: &mut Runtime<Ui>) {
    rt.update(|ui, _| ui.review_volume_hover(|hover| hover.button = true));
    rt.frame(1.0 / 60.0);
}

fn drag_slider_to(rt: &mut Runtime<Ui>, share: f32) {
    let [x, y, w, h] = rect(rt, "volume-slider");
    let at = (x + w * share, y + h / 2.0);
    rt.pointer_down(at.0, at.1);
    rt.pointer_up(at.0, at.1);
    rt.frame(1.0 / 60.0);
}

#[test]
fn the_popover_is_closed_until_the_pointer_is_over_the_volume_button() {
    let mut rt = review_runtime();
    assert!(rt.find("volume-slider").is_none());
    open_popover(&mut rt);
    assert!(rt.find("volume-slider").is_some());
    rt.update(|ui, _| ui.review_volume_hover(|hover| hover.button = false));
    rt.frame(1.0 / 60.0);
    assert!(rt.find("volume-slider").is_none());
}

#[test]
fn the_popover_stays_open_while_the_pointer_is_inside_it_or_dragging() {
    let mut rt = review_runtime();
    open_popover(&mut rt);
    rt.update(|ui, _| {
        ui.review_volume_hover(|hover| {
            hover.button = false;
            hover.popover = true;
        })
    });
    rt.frame(1.0 / 60.0);
    assert!(rt.find("volume-slider").is_some());
    rt.update(|ui, _| {
        ui.review_volume_hover(|hover| {
            hover.popover = false;
            hover.dragging = true;
        })
    });
    rt.frame(1.0 / 60.0);
    assert!(rt.find("volume-slider").is_some());
}

#[test]
fn the_slider_sets_the_volume_and_tells_the_player() {
    let mut rt = review_runtime();
    open_popover(&mut rt);
    drag_slider_to(&mut rt, 0.5);
    let set = volume(&rt);
    assert!((set - 0.5).abs() < 0.08, "{set}");
    assert!(!muted(&rt));
    assert!(fake_player::calls().contains(&Call::Volume(set)));
}

#[test]
fn the_slider_ends_reach_silence_and_full_volume() {
    let mut rt = review_runtime();
    open_popover(&mut rt);
    drag_slider_to(&mut rt, 0.98);
    assert_eq!(volume(&rt), 1.0);
    drag_slider_to(&mut rt, 0.02);
    assert_eq!(volume(&rt), 0.0);
    assert!(muted(&rt));
}

#[test]
fn the_mute_button_keeps_the_volume_and_the_slider_shows_silence() {
    let mut rt = review_runtime();
    open_popover(&mut rt);
    drag_slider_to(&mut rt, 0.5);
    let kept = volume(&rt);
    tap(&mut rt, "review-mute");
    assert!(muted(&rt));
    assert_eq!(volume(&rt), kept);
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.shown_volume()),
        Some(0.0)
    );
    tap(&mut rt, "review-mute");
    assert!(!muted(&rt));
    assert_eq!(volume(&rt), kept);
}

#[test]
fn unmuting_from_silence_restores_a_sensible_volume() {
    let mut rt = review_runtime();
    rt.update(|ui, _| ui.review_set_volume(0.0));
    refresh(&mut rt);
    fake_player::clear_calls();
    tap(&mut rt, "review-mute");
    assert!(!muted(&rt));
    assert_eq!(volume(&rt), 0.5);
    assert_eq!(
        fake_player::calls(),
        vec![Call::Volume(0.5), Call::Muted(false)]
    );
}

#[test]
fn the_arrow_keys_step_the_volume_by_five_percent() {
    let mut rt = review_runtime();
    rt.update(|ui, _| ui.review_step_volume(-1));
    assert!((volume(&rt) - 0.95).abs() < 1e-6);
    rt.update(|ui, _| ui.review_step_volume(10));
    assert_eq!(volume(&rt), 1.0);
}

#[test]
fn the_volume_carries_into_the_next_review_and_the_player() {
    let mut rt = review_runtime();
    rt.update(|ui, _| ui.review_set_volume(0.3));
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    rt.frame(1.0 / 60.0);
    assert_eq!(volume(&rt), 0.3);
    assert!(fake_player::calls().contains(&Call::Volume(0.3)));
}

#[test]
fn an_open_menu_keeps_the_theater_controls_on_screen() {
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    ui.open_review_entry(review_entry());
    fake_player::set_playing(true);
    let review = ui.review.as_mut().expect("review");
    review.theater = true;
    review.speed_menu = true;
    assert!(!review.hide_idle_controls(60.0));
    review.speed_menu = false;
    assert!(review.hide_idle_controls(60.0));
    fake_player::set_playing(false);
}
