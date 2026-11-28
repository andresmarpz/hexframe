use greyscreen_core::game::GameResult;
use jiff::Zoned;
use rune::{El, TextStyle, col, el, row, text};
use rune_kit::Kit;

use super::surface::{header, plural, sized, surface};
use crate::ui::Ui;
use crate::ui::library::format::{week_stats, week_strip};
use crate::view::TABULAR;

const STRIP_LIMIT: usize = 12;
const SQUARE: f32 = 14.0;
const CARD_HEIGHT: f32 = 141.0;

pub(super) fn section(s: &Ui, k: &Kit, now: &Zoned) -> Option<El> {
    let stats = week_stats(&s.recordings, now);
    if stats.games == 0 {
        return None;
    }
    let strip = week_strip(&s.recordings, now, STRIP_LIMIT);
    let rate = stats
        .win_rate()
        .map_or_else(|| "—".to_owned(), |rate| format!("{rate}%"));
    let mut summary = format!(
        "{} · {}",
        plural(stats.wins, "win", "wins"),
        plural(stats.losses, "loss", "losses")
    );
    if let Some(champion) = &strip.most_played {
        summary.push_str(&format!(" · Most played: {champion}"));
    }
    let big = TextStyle {
        weight: 600.0,
        tracking: -0.64,
        ..sized(k.ty.body_medium.features(TABULAR), 32.0, 36.0)
    };
    let figure = row()
        .gap(8.0)
        .center_y()
        .child(
            text(rate)
                .key("week-win-rate")
                .font(big)
                .color(k.c.text_base),
        )
        .child(
            text("win rate")
                .font(sized(k.ty.ui, 13.0, 18.0))
                .color(k.c.text_muted),
        );
    let mut squares = row().key("week-strip").gap(4.0);
    for result in &strip.results {
        squares = squares.child(square(k, *result));
    }
    let card = surface(k)
        .key("this-week-card")
        .h(CARD_HEIGHT)
        .px(16.0)
        .gap(14.0)
        .justify_center()
        .child(figure)
        .child(squares)
        .child(
            text(summary)
                .key("week-summary")
                .font(sized(k.ty.ui, 12.0, 16.0).features(TABULAR))
                .color(k.c.text_muted),
        );
    Some(
        col()
            .key("this-week")
            .w(0.0)
            .grow(1.0)
            .min_w(0.0)
            .gap(8.0)
            .child(header(
                k,
                "This week",
                Some(plural(stats.games, "game", "games")),
            ))
            .child(card),
    )
}

fn square(k: &Kit, result: GameResult) -> El {
    let color = match result {
        GameResult::Win => k.c.success.fg,
        GameResult::Loss => k.c.danger.fg,
        GameResult::Unknown => k.c.text_faint,
    };
    el().size(SQUARE, SQUARE).radius(k.radius.xs).fill(color)
}
