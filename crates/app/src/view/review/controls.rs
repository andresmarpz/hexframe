use std::time::Duration;

use rune::{Color, Drag, El, Phase, el, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Menu, MenuItem,
    Placement, Popover,
};

use super::panel::filter;
use super::{icons, tabular};
use crate::ui::{NORMAL_SPEED, ReviewState, SPEEDS, Ui};

const SPEED_WIDTH: f32 = 60.0;
const SLIDER_HEIGHT: f32 = 20.0;
const TRACK_HEIGHT: f32 = 4.0;
const THUMB: f32 = 12.0;
const BODY_PADDING: f32 = 12.0;
const BODY_GAP: f32 = 12.0;
const PERCENT_WIDTH: f32 = 32.0;
const VOLUME_STEP: f32 = 0.05;
const HOVER_ENTER: Duration = Duration::from_millis(80);
const HOVER_LEAVE: Duration = Duration::from_millis(250);

fn speed_label(rate: f32) -> String {
    format!("{rate}×")
}

fn speed_name(index: usize) -> String {
    let label = speed_label(SPEEDS[index]);
    if index == NORMAL_SPEED {
        format!("{label} (Normal)")
    } else {
        label
    }
}

fn speed_menu(open: bool) -> Menu {
    Menu::new("speed-menu")
        .open(open)
        .placement(Placement::TopEnd)
        .min_width(144.0)
        .on_close(|ui: &mut Ui| ui.review_speed_menu(false))
}

pub(super) fn speed_trigger(review: &ReviewState, k: &Kit) -> El {
    let open = review.speed_menu;
    let label = speed_label(review.speed_rate());
    let button = Button::new(label.clone())
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .help("Playback speed")
        .expanded(open)
        .label_with(move |font, ink| text(label).font(tabular(font)).color(ink))
        .on_press(move |ui: &mut Ui| ui.review_speed_menu(!open))
        .build(k)
        .w(SPEED_WIDTH)
        .key("review-speed");
    speed_menu(open).trigger(button).build(k, &mut Vec::new())
}

fn speed_overlay(review: &ReviewState, k: &Kit, layer: &mut Vec<El>) {
    if !review.speed_menu {
        return;
    }
    let mut menu = speed_menu(true);
    for index in 0..SPEEDS.len() {
        menu = menu.item(
            MenuItem::new(speed_name(index))
                .checked(index == review.speed)
                .on_select(move |ui: &mut Ui| ui.review_set_speed(index)),
        );
    }
    menu.build(k, layer);
}

fn percent(volume: f32) -> u32 {
    (volume * 100.0).round() as u32
}

pub(super) fn volume_at(x: f32, width: f32) -> f32 {
    let travel = width - THUMB;
    if travel <= 0.0 {
        return 0.0;
    }
    let value = ((x - THUMB / 2.0) / travel).clamp(0.0, 1.0);
    (value * 100.0).round() / 100.0
}

fn slider(review: &ReviewState, k: &Kit) -> El {
    let value = review.shown_volume();
    let track = el()
        .key("volume-track")
        .pin(
            Some(0.0),
            Some((SLIDER_HEIGHT - TRACK_HEIGHT) / 2.0),
            Some(0.0),
            None,
        )
        .h(TRACK_HEIGHT)
        .radius(TRACK_HEIGHT / 2.0)
        .fill(k.c.layer_03);
    let knob = row()
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .pr(THUMB)
        .center_y()
        .child(
            el().key("volume-fill")
                .w_frac(value)
                .h(TRACK_HEIGHT)
                .radius(TRACK_HEIGHT / 2.0)
                .fill(k.c.bg_accent),
        )
        .child(
            el().key("volume-thumb")
                .size(THUMB, THUMB)
                .radius(THUMB / 2.0)
                .fill(Color::WHITE)
                .border(1.0, k.c.border_strong),
        );
    el().key("volume-slider")
        .label("Volume")
        .range_value(f64::from(percent(value)), 0.0, 100.0)
        .range_step(f64::from(percent(VOLUME_STEP)))
        .grow(1.0)
        .h(SLIDER_HEIGHT)
        .on_step(|ui: &mut Ui, delta, _| ui.review_step_volume(delta))
        .on_step_limit(|ui: &mut Ui, max, _| ui.review_set_volume(if max { 1.0 } else { 0.0 }))
        .on_drag(|ui: &mut Ui, drag: &mut Drag, _| {
            let dragging = drag.phase != Phase::Up;
            ui.review_volume_hover(|hover| hover.dragging = dragging);
            if dragging {
                ui.review_set_volume(volume_at(drag.x, drag.w));
            }
        })
        .child(track)
        .child(knob)
}

fn popover_body(review: &ReviewState, k: &Kit) -> El {
    row()
        .key("volume-body")
        .w(200.0)
        .padding(BODY_PADDING)
        .gap(BODY_GAP)
        .center_y()
        .on_hover_change(|ui: &mut Ui, inside, _| ui.review_volume_hover(|h| h.popover = inside))
        .hover_delays(HOVER_ENTER, HOVER_LEAVE)
        .child(slider(review, k))
        .child(
            text(percent(review.shown_volume()).to_string())
                .key("volume-percent")
                .font(tabular(k.ty.small_medium))
                .color(k.c.text_muted)
                .w(PERCENT_WIDTH),
        )
}

fn volume_popover(open: bool) -> Popover {
    Popover::new("volume")
        .label("Volume")
        .padding(0.0)
        .placement(Placement::Top)
        .gutter(4.0)
        .open(open)
}

pub(super) fn volume_trigger(review: &ReviewState, k: &Kit) -> El {
    let open = review.volume_open();
    let (icon, name) = if review.shown_volume() <= 0.0 {
        (&icons::MUTED, "Unmute")
    } else {
        (&icons::VOLUME, "Mute")
    };
    let mut button = IconButton::new(icon)
        .label(name)
        .shortcut("M")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(|ui: &mut Ui| ui.review_mute());
    if !open {
        button = button.help(name);
    }
    let trigger = button
        .build(k)
        .key("review-mute")
        .on_hover_change(|ui: &mut Ui, inside, _| ui.review_volume_hover(|h| h.button = inside))
        .hover_delays(HOVER_ENTER, HOVER_LEAVE);
    volume_popover(open)
        .trigger(trigger)
        .build(k, &mut Vec::new())
}

fn volume_overlay(review: &ReviewState, k: &Kit, layer: &mut Vec<El>) {
    if review.volume_open() {
        volume_popover(true)
            .child(popover_body(review, k))
            .build(k, layer);
    }
}

pub(crate) fn layer(ui: &Ui, k: &Kit, layer: &mut Vec<El>) {
    let Some(review) = ui.review.as_ref().filter(|r| r.controls_menu_open()) else {
        return;
    };
    let dark = Kit::neutral(true);
    let k = if review.theater { &dark } else { k };
    speed_overlay(review, k, layer);
    volume_overlay(review, k, layer);
    filter::overlay(review, k, layer);
}
