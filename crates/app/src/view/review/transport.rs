use rune::{El, Tx, col, el, row};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit};

use super::{clock_text, controls, icons, play_button, timeline};
use crate::ui::{ReviewState, Ui};

const SKIP_SECS: f64 = 5.0;
const SHOW_SKIP_FROM: f32 = 440.0;
const SHOW_SPEED_FROM: f32 = 360.0;
const SHOW_CLOCK_FROM: f32 = 260.0;

struct Visible {
    skip: bool,
    speed: bool,
    clock: bool,
}

fn visible(width: Option<f32>) -> Visible {
    let at_least = |min: f32| width.is_none_or(|w| w >= min);
    Visible {
        skip: at_least(SHOW_SKIP_FROM),
        speed: at_least(SHOW_SPEED_FROM),
        clock: at_least(SHOW_CLOCK_FROM),
    }
}

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

fn export_button(k: &Kit) -> El {
    IconButton::new(&crate::view::widgets::EXPORT)
        .label("Export this moment")
        .help("Export this moment…")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(|ui: &mut Ui| ui.open_export_moment())
        .build(k)
        .key("review-export")
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

pub(super) fn transport(review: &ReviewState, k: &Kit, theater: bool, width: Option<f32>) -> El {
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
    let show = visible(width);
    let mut buttons = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(play_button(review, k, variant));
    if show.skip {
        buttons = buttons
            .child(skip_button(false, k))
            .child(skip_button(true, k));
    }
    if show.clock {
        buttons = buttons.child(clock_text(review, k, clock));
    }
    buttons = buttons.child(el().grow(1.0));
    if !theater && show.speed && review.can_export_moment() {
        buttons = buttons.child(export_button(k));
    }
    if show.speed {
        buttons = buttons.child(controls::speed_trigger(review, k));
    }
    let buttons = buttons
        .child(controls::volume_trigger(review, k))
        .child(screen_button(theater, k));
    col()
        .key("review-controls")
        .w_full()
        .gap(2.0)
        .child(timeline::timeline(review, k))
        .child(buttons)
}
