use greyscreen_core::game::GameResult;
use greyscreen_engine::RecordingEntry;
use rune::{Color, El, TextStyle, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::feed::tinted_pill;
use crate::ui::library_format::{Kda, kda, mode_label};
use crate::ui::{Ui, format_clock};
use crate::view::champion::champion_icon;
use crate::view::{card, hairline};

const ICON_SIZE: f32 = 40.0;
const TABULAR: &str = "'tnum' 1";

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
        .child(champion_icon(k, champion, ICON_SIZE))
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
        .child(open);
    card(k)
        .key("last-game-card")
        .child(
            row().w_full().h(44.0).px(16.0).center_y().child(
                text("Last game")
                    .font(k.ty.body_medium)
                    .color(k.c.text_base),
            ),
        )
        .child(hairline(k))
        .child(summary)
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
