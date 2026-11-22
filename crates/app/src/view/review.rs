mod card;
mod controls;
mod death_summary;
mod deaths;
mod health;
mod icons;
mod panel;
mod seek_preview;
mod theater;
mod timeline;
mod toolbar;
mod transport;

#[cfg(test)]
mod tests;

use rune::{Color, El, PlayerState, ScrollSpec, TextStyle, Tx, col, el, row, text};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit};

use super::TABULAR;
use crate::ui::{ReviewState, Ui, format_clock};

const PAGE_PADDING: f32 = 16.0;
const STACK_BELOW: f32 = 800.0;
const VIDEO_ASPECT: f32 = 16.0 / 9.0;
const COLUMN_GAP: f32 = 16.0;
const NO_VIDEO: &str = "No video for this game";

pub(super) use card::hover_card;
pub(super) use controls::layer as controls_layer;
pub(super) use seek_preview::seek_preview;
pub(super) use theater::screen as theater;
pub(super) use toolbar::{result_label, toolbar};

pub(super) fn tabular(style: TextStyle) -> TextStyle {
    if style.features.is_empty() {
        style.features(TABULAR)
    } else {
        style
    }
}

pub(super) fn tone_pill(k: &Kit, label: &str, colors: rune_kit::StateColors) -> El {
    row()
        .h(22.0)
        .px(10.0)
        .center()
        .radius(11.0)
        .fill(colors.bg)
        .child(
            text(label.to_owned())
                .font(k.ty.small_medium)
                .color(colors.fg),
        )
}

pub(super) fn play_button(review: &ReviewState, k: &Kit, variant: IconButtonVariant) -> El {
    let (icon, name) = if review.is_playing() {
        (&icons::PAUSE, "Pause")
    } else {
        (&icons::PLAY, "Play")
    };
    IconButton::new(icon)
        .label(name)
        .help(name)
        .shortcut("Space")
        .variant(variant)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| {
            if let Some(review) = &mut ui.review {
                review.toggle();
            }
        })
        .build(k)
        .key("review-play")
}

pub(super) fn clock_text(review: &ReviewState, k: &Kit, color: Color) -> El {
    let clock = format!(
        "{} / {}",
        format_clock(review.time() as u32),
        format_clock(review.duration() as u32)
    );
    text(clock)
        .key("review-clock")
        .font(tabular(k.ty.body))
        .color(color)
}

pub(super) fn page(s: &Ui, k: &Kit, width: Option<f32>) -> El {
    let Some(review) = &s.review else {
        return col().key("review").w_full().h_full();
    };
    match &review.death {
        Some(state) if state.summary => death_summary::screen(review, state, k),
        Some(state) => deaths::screen(review, state, &s.death_note, k),
        None => screen(review, k, width),
    }
}

pub(super) fn is_stacked(width: Option<f32>) -> bool {
    width.is_some_and(|w| w < STACK_BELOW)
}

fn screen(review: &ReviewState, k: &Kit, width: Option<f32>) -> El {
    let stacked = is_stacked(width);
    let inner = width.map(|w| w - 2.0 * PAGE_PADDING);
    let body = if stacked {
        col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .gap(COLUMN_GAP)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(player_column(review, k, true, inner))
            .child(panel::panel(review, k, true))
    } else {
        let player = inner.map(|w| w - COLUMN_GAP - panel::PANEL_WIDTH);
        row()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .gap(COLUMN_GAP)
            .child(player_column(review, k, false, player))
            .child(panel::panel(review, k, false))
    };
    col()
        .key("review")
        .w_full()
        .h_full()
        .min_h(0.0)
        .padding(PAGE_PADDING)
        .child(body)
}

fn player_column(review: &ReviewState, k: &Kit, stacked: bool, width: Option<f32>) -> El {
    let column = col().key("review-player").min_w(0.0).gap(8.0);
    let column = if stacked {
        column.w_full()
    } else {
        column.grow(1.0).min_h(0.0)
    };
    column
        .child(video(review, k, stacked))
        .child(transport::transport(review, k, false, width))
}

fn video(review: &ReviewState, k: &Kit, stacked: bool) -> El {
    let frame = el()
        .key("review-video")
        .label("Game video")
        .focusable(true)
        .w_full()
        .min_h(0.0)
        .radius(8.0)
        .fill(Color::BLACK);
    let frame = if stacked {
        frame.aspect_ratio(VIDEO_ASPECT)
    } else {
        frame.grow(1.0)
    };
    video_frame(review, k, frame)
}

fn video_frame(review: &ReviewState, k: &Kit, frame: El) -> El {
    let frame = frame
        .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.video_click(tx))
        .on_double_press_tx(|ui: &mut Ui, tx: &mut Tx| {
            let on = !ui.in_theater();
            ui.set_theater(on, tx);
        });
    let message = |label: String| {
        frame
            .clone()
            .center()
            .child(text(label).font(k.ty.body).color(k.c.text_muted))
    };
    match &review.player {
        None => message(NO_VIDEO.into()),
        Some(player) => match player.state() {
            PlayerState::Failed(err) => message(err),
            _ => player.show(frame),
        },
    }
}
