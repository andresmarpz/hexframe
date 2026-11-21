use greyscreen_core::game::GameResult;
use greyscreen_engine::RecordingEntry;
use rune::{Color, El, TextStyle, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::feed::tinted_pill;
use crate::ui::library::format::{Kda, death_total, kda, mode_label};
use crate::ui::{Ui, format_clock};
use crate::view::thumb;
use crate::view::{TABULAR, titled_card};

const THUMB_WIDTH: f32 = 96.0;

pub(super) fn panel(entry: &RecordingEntry, k: &Kit) -> El {
    let game = &entry.meta.game;
    let champion = game.info.me.champion.as_str();
    let events = entry.meta.events.len();
    let detail = format!(
        "{} · {} · {} {}",
        mode_label(&game.info.mode),
        format_clock(entry.meta.duration_secs as u32),
        events,
        if events == 1 { "event" } else { "events" },
    );
    let open = Button::new("Open")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.open_last_game())
        .build(k)
        .key("open-last-game");
    let summary = row()
        .w_full()
        .px(16.0)
        .py(14.0)
        .gap(12.0)
        .center_y()
        .child(thumb::game_thumb(k, entry, THUMB_WIDTH))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(champion)
                        .font(k.ty.body_medium)
                        .color(k.c.text_base)
                        .truncate(),
                )
                .child(text(detail).font(k.ty.small).color(k.c.text_muted)),
        )
        .child(kda_figures(kda(&entry.meta.events, &game.info.me), k))
        .child(result_pill(game.result, k))
        .children(deaths_button(entry, k))
        .child(open);
    titled_card(k, "last-game-card", "Last game").child(summary)
}

fn deaths_button(entry: &RecordingEntry, k: &Kit) -> Option<El> {
    let count = death_total(entry);
    if count == 0 {
        return None;
    }
    let id = entry.meta.id.clone();
    Some(
        Button::new(format!("Review deaths ({count})"))
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Large)
            .on_press(move |ui: &mut Ui| ui.review_deaths_of(id.clone()))
            .build(k)
            .key("last-game-deaths"),
    )
}

fn result_pill(result: GameResult, k: &Kit) -> El {
    let (label, ink) = match result {
        GameResult::Win => ("VICTORY", k.c.success.icon),
        GameResult::Loss => ("DEFEAT", k.c.danger.icon),
        GameResult::Unknown => ("UNKNOWN", k.c.text_muted),
    };
    tinted_pill(k, label.to_owned(), ink).key("last-game-result")
}

fn kda_figures(kda: Kda, k: &Kit) -> El {
    let font = TextStyle {
        size: 14.0,
        line_height: 20.0,
        ..k.ty.ui_medium.features(TABULAR)
    };
    let figure = |n: u32, color: Color| text(n.to_string()).font(font).color(color);
    let slash = || text(" / ").font(font).color(k.c.text_faint);
    row()
        .key("last-game-kda")
        .child(figure(kda.kills, k.c.text_base))
        .child(slash())
        .child(figure(kda.deaths, k.c.danger.fg))
        .child(slash())
        .child(figure(kda.assists, k.c.text_base))
}
