use greyscreen_engine::RecordingEntry;
use jiff::Zoned;
use rune::{Cursor, El, TextStyle, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

use crate::ui::library_format::{kda, mode_label, recent_games, when};
use crate::ui::{Screen, Ui};
use crate::view::champion::champion_icon;
use crate::view::recordings::{kda_cell, result_cell};
use crate::view::{card, hairline};

pub(super) const LIMIT: usize = 5;
const ROW_HEIGHT: f32 = 52.0;
const ICON_SIZE: f32 = 32.0;
const COL_RESULT: f32 = 64.0;
const COL_KDA: f32 = 72.0;
const COL_WHEN: f32 = 116.0;
const TABULAR: &str = "'tnum' 1";

pub(super) fn section(s: &Ui, k: &Kit, now: &Zoned) -> El {
    let view_all = Button::new("View all")
        .variant(ButtonVariant::Ghost)
        .on_press(|ui: &mut Ui| ui.go(Screen::Recordings))
        .build(k)
        .key("view-all-games");
    let header = row()
        .w_full()
        .h(44.0)
        .pl(16.0)
        .pr(8.0)
        .center_y()
        .justify_between()
        .child(
            text("Recent games")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(view_all);
    let rows = recent_games(&s.recordings, LIMIT)
        .into_iter()
        .map(|rec| game_row(rec, now, k));
    card(k)
        .key("recent-games")
        .child(header)
        .child(hairline(k))
        .child(col().w_full().py(4.0).children(rows))
}

fn game_row(rec: &RecordingEntry, now: &Zoned, k: &Kit) -> El {
    let game = &rec.meta.game;
    let champion = game.info.me.champion.as_str();
    let id = rec.meta.id.clone();
    let shown = when(&rec.meta.started_at, now);
    let figures = TextStyle {
        ..k.ty.ui.features(TABULAR)
    };
    row()
        .key(format!("home-game-{id}"))
        .role("button")
        .label(format!("Review {champion} game"))
        .w_full()
        .h(ROW_HEIGHT)
        .px(16.0)
        .gap(12.0)
        .center_y()
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .on_press(move |ui: &mut Ui| ui.open_review(id.clone()))
        .child(champion_icon(k, champion, ICON_SIZE))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(
                    text(champion.to_owned())
                        .font(k.ty.body_medium)
                        .color(k.c.text_base)
                        .truncate(),
                )
                .child(
                    text(mode_label(&game.info.mode))
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .truncate(),
                ),
        )
        .child(
            row()
                .w(COL_RESULT)
                .center_y()
                .child(result_cell(game.result, k)),
        )
        .child(
            row()
                .w(COL_KDA)
                .center_y()
                .justify_end()
                .child(kda_cell(kda(&rec.meta.events, &game.info.me), k)),
        )
        .child(
            row().w(COL_WHEN).center_y().justify_end().child(
                text(format!("{} {}", shown.day, shown.time))
                    .font(figures)
                    .color(k.c.text_muted),
            ),
        )
}
