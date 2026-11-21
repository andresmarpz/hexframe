use rune::{Color, El, Tx, col, el, row, text};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit};

use super::{clock_text, icons, play_button, tabular, timeline};
use crate::ui::{ReviewState, Ui};

const SKIP_SECS: f64 = 5.0;
const SPEED_WIDTH: f32 = 52.0;
const SPEED_HEIGHT: f32 = 28.0;

fn skip_button(forward: bool, k: &Kit) -> El {
    let (icon, name, key, shortcut) = if forward {
        (
            &icons::FORWARD,
            "Forward 5 seconds",
            "review-forward",
            "Right",
        )
    } else {
        (&icons::REWIND, "Back 5 seconds", "review-rewind", "Left")
    };
    let delta = if forward { SKIP_SECS } else { -SKIP_SECS };
    IconButton::new(icon)
        .label(name)
        .help(name)
        .shortcut(shortcut)
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(move |ui: &mut Ui| ui.review_step(delta))
        .build(k)
        .key(key)
}

fn speed_label(rate: f32) -> String {
    format!("{rate}×")
}

fn speed_pill(review: &ReviewState, k: &Kit, theater: bool) -> El {
    let fill = if theater {
        Color::WHITE.alpha(0.12)
    } else {
        k.c.layer_02
    };
    let label = speed_label(review.speed_rate());
    let pill = row()
        .key("review-speed")
        .role("button")
        .label(format!("Playback speed {label}"))
        .w(SPEED_WIDTH)
        .h(SPEED_HEIGHT)
        .center()
        .radius(SPEED_HEIGHT / 2.0)
        .fill(fill)
        .on_press(|ui: &mut Ui| ui.review_cycle_speed())
        .child(
            text(label)
                .font(tabular(k.ty.small_medium))
                .color(k.c.text_base),
        );
    k.interactive(pill)
}

fn mute_button(review: &ReviewState, k: &Kit) -> El {
    let (icon, name) = if review.muted {
        (&icons::MUTED, "Unmute")
    } else {
        (&icons::VOLUME, "Mute")
    };
    IconButton::new(icon)
        .label(name)
        .help(name)
        .shortcut("M")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(|ui: &mut Ui| ui.review_mute())
        .build(k)
        .key("review-mute")
}

fn screen_button(theater: bool, k: &Kit) -> El {
    let (icon, name, key) = if theater {
        (&icons::SHRINK, "Exit theater", "theater-shrink")
    } else {
        (&icons::EXPAND, "Theater", "review-theater")
    };
    IconButton::new(icon)
        .label(name)
        .help(name)
        .shortcut("F")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| ui.set_theater(!theater, tx))
        .build(k)
        .key(key)
}

pub(super) fn transport(review: &ReviewState, k: &Kit, theater: bool) -> El {
    let variant = if theater {
        IconButtonVariant::Ghost
    } else {
        IconButtonVariant::Neutral
    };
    let clock = if theater {
        k.c.text_base
    } else {
        k.c.text_muted
    };
    let buttons = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(play_button(review, k, variant))
        .child(skip_button(false, k))
        .child(skip_button(true, k))
        .child(clock_text(review, k, clock))
        .child(el().grow(1.0))
        .child(speed_pill(review, k, theater))
        .child(mute_button(review, k))
        .child(screen_button(theater, k));
    col()
        .key("review-controls")
        .w_full()
        .gap(2.0)
        .child(timeline::timeline(review, k))
        .child(buttons)
}
