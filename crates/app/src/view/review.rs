mod icons;
mod panel;
mod theater;
mod timeline;

#[cfg(test)]
mod tests;

use greyscreen_core::game::GameResult;
use rune::{Color, El, PlayerState, Tx, col, el, row, text};
use rune_kit::{
    Badge, BadgeVariant, Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit,
};

use super::title_style;
use crate::ui::{ReviewState, Screen, Ui, format_clock, format_date};

const VIDEO_RATIO: f32 = 16.0 / 9.0;
const NO_VIDEO: &str = "No video for this game";

pub(super) use theater::screen as theater;

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    match &s.review {
        Some(review) => screen(review, k),
        None => col().key("review").w_full().h_full(),
    }
}

fn screen(review: &ReviewState, k: &Kit) -> El {
    col()
        .key("review")
        .w_full()
        .h_full()
        .min_h(0.0)
        .gap(16.0)
        .child(header(review, k))
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .gap(16.0)
                .child(player_column(review, k))
                .child(panel::panel(review, k)),
        )
}

fn header(review: &ReviewState, k: &Kit) -> El {
    let meta = &review.entry.meta;
    let back = Button::new("Back")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.go(Screen::Recordings))
        .build(k)
        .key("review-back");
    let result = match meta.game.result {
        GameResult::Win => Some(Badge::new("WIN").variant(BadgeVariant::Accent)),
        GameResult::Loss => Some(Badge::new("LOSS").variant(BadgeVariant::Subtle)),
        GameResult::Unknown => None,
    };
    let mut line = row()
        .key("review-header")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(back)
        .child(
            text(meta.game.info.me.champion.clone())
                .font(title_style(k))
                .color(k.c.text_base),
        )
        .child(
            Badge::new(meta.game.info.mode.clone())
                .variant(BadgeVariant::Neutral)
                .build(k),
        );
    if let Some(result) = result {
        line = line.child(result.build(k));
    }
    line.child(
        text(format_date(&meta.started_at))
            .font(k.ty.body)
            .color(k.c.text_muted),
    )
}

fn player_column(review: &ReviewState, k: &Kit) -> El {
    col()
        .key("review-player")
        .grow(1.0)
        .min_w(0.0)
        .gap(12.0)
        .child(video(review, k))
        .child(controls(review, k))
}

fn video(review: &ReviewState, k: &Kit) -> El {
    let frame = el()
        .key("review-video")
        .w_full()
        .aspect_ratio(VIDEO_RATIO)
        .radius(8.0)
        .fill(Color::BLACK);
    video_frame(review, k, frame)
}

fn video_frame(review: &ReviewState, k: &Kit, frame: El) -> El {
    let frame = frame.on_double_press_tx(|ui: &mut Ui, tx: &mut Tx| {
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

fn controls(review: &ReviewState, k: &Kit) -> El {
    let label = if review.is_playing() { "Pause" } else { "Play" };
    let play = Button::new(label)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Regular)
        .on_press(|ui: &mut Ui| {
            if let Some(review) = &ui.review {
                review.toggle();
            }
        })
        .build(k)
        .key("review-play");
    let theater = IconButton::new(&icons::EXPAND)
        .label("Theater")
        .help("Theater")
        .shortcut("F")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.set_theater(true, tx))
        .build(k)
        .key("review-theater");
    let clock = format!(
        "{} / {}",
        format_clock(review.time() as u32),
        format_clock(review.duration() as u32)
    );
    row()
        .key("review-controls")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(play)
        .child(theater)
        .child(
            text(clock)
                .key("review-clock")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
        .child(timeline::timeline(review, k))
}
