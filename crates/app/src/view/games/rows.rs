use greyscreen_core::game::GameResult;
use greyscreen_engine::{EngineStatus, RecordingEntry};
use jiff::Zoned;
use rune::{Cursor, El, TextStyle, Tx, col, row, text};
use rune_kit::Kit;

use super::{ROW_HEIGHT, TABULAR, dot};
use crate::ui::library::format::{Kda, kda, mode_label, review_progress, when};
use crate::ui::{Ui, format_clock};
use crate::view::champion::champion_icon;
use crate::view::thumb;

const ICON_SIZE: f32 = 32.0;
const THUMB_WIDTH: f32 = 72.0;
const TINT: f32 = 0.1;
const TINT_SELECTED: f32 = 0.2;

fn base(k: &Kit, key: String, label: String, selected: bool) -> El {
    let line = row()
        .key(key)
        .role("button")
        .label(label)
        .w_full()
        .h(ROW_HEIGHT)
        .px(8.0)
        .gap(10.0)
        .center_y()
        .radius(k.radius.md)
        .cursor(Cursor::Pointer)
        .focusable(false)
        .preserve_focus_on_press(true);
    let line = k.interactive(line);
    if selected {
        line.fill(k.c.pressed)
    } else {
        line
    }
}

fn two_lines(top: [El; 2], bottom: [El; 2]) -> El {
    let [top_left, top_right] = top;
    let [bottom_left, bottom_right] = bottom;
    col()
        .grow(1.0)
        .min_w(0.0)
        .gap(1.0)
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(top_left)
                .child(top_right),
        )
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(bottom_left)
                .child(bottom_right),
        )
}

fn grow(label: El) -> El {
    label.grow(1.0).min_w(0.0)
}

fn kda_text(kda: Kda, k: &Kit) -> El {
    let font = k.ty.ui.features(TABULAR);
    let figure = |n: u32, color| text(n.to_string()).font(font).color(color);
    let slash = || text(" / ").font(font).color(k.c.text_faint);
    row()
        .child(figure(kda.kills, k.c.text_base))
        .child(slash())
        .child(figure(kda.deaths, k.c.danger.fg))
        .child(slash())
        .child(figure(kda.assists, k.c.text_base))
}

fn result_text(result: GameResult, k: &Kit) -> El {
    let (label, color) = match result {
        GameResult::Win => ("Victory", k.c.success.fg),
        GameResult::Loss => ("Defeat", k.c.danger.fg),
        GameResult::Unknown => ("", k.c.text_muted),
    };
    text(label).font(k.ty.small_medium).color(color)
}

pub(super) fn game(rec: &RecordingEntry, selected: bool, k: &Kit) -> El {
    let info = &rec.meta.game.info;
    let champion = info.me.champion.as_str();
    let kda = kda(&rec.meta.events, &info.me);
    let shown = when(&rec.meta.started_at, &Zoned::now());
    let mode = mode_label(&info.mode);
    let result = rec.meta.game.result;
    let result_word = match result {
        GameResult::Win => "Victory, ",
        GameResult::Loss => "Defeat, ",
        GameResult::Unknown => "",
    };
    let progress = review_progress(rec);
    let label = format!(
        "{champion}, {mode}, {result_word}{}/{}/{}, {}{}",
        kda.kills,
        kda.deaths,
        kda.assists,
        shown.time,
        progress
            .as_ref()
            .map_or_else(String::new, |p| format!(", {p}"))
    );
    let id = rec.meta.id.clone();
    let menu_id = id.clone();
    let name = text(champion.to_owned())
        .font(k.ty.body_medium)
        .color(k.c.text_base)
        .truncate();
    let detail_line = match &progress {
        Some(progress) => format!("{mode} · {} · {progress}", shown.time),
        None => format!("{mode} · {}", shown.time),
    };
    let detail = text(detail_line)
        .font(k.ty.small.features(TABULAR))
        .color(k.c.text_muted)
        .truncate();
    base(k, rec.meta.id.clone(), label, selected)
        .on_press(move |ui: &mut Ui| ui.open_review(id.clone()))
        .on_secondary_press(move |ui: &mut Ui, at, _: &mut Tx| {
            ui.open_row_menu(menu_id.clone(), at)
        })
        .child(thumb::game_thumb(k, rec, THUMB_WIDTH))
        .child(two_lines(
            [grow(name), kda_text(kda, k)],
            [grow(detail), result_text(result, k)],
        ))
}

pub(super) fn live(s: &Ui, k: &Kit) -> El {
    let (title, mode, icon, saving) = match &s.status {
        EngineStatus::Recording {
            game: Some(info), ..
        } => (
            info.me.champion.clone(),
            mode_label(&info.mode),
            champion_icon(k, &info.me.champion, ICON_SIZE),
            false,
        ),
        status => (
            "Test recording".to_owned(),
            "Primary screen".to_owned(),
            champion_icon(k, "Test", ICON_SIZE),
            matches!(status, EngineStatus::Finalizing),
        ),
    };
    let saving = saving || matches!(s.status, EngineStatus::Finalizing);
    let (word, color) = if saving {
        ("Saving…", k.c.warning.fg)
    } else {
        ("Recording", k.c.danger.fg)
    };
    let tint = if s.live_selected { TINT_SELECTED } else { TINT };
    let clock = text(format_clock(s.elapsed_secs))
        .font(TextStyle {
            features: TABULAR,
            ..k.ty.ui_medium
        })
        .color(k.c.text_base);
    let status = row()
        .gap(6.0)
        .center_y()
        .child(dot(color, true))
        .child(text(word).font(k.ty.small_medium).color(color));
    base(
        k,
        "live-row".to_owned(),
        format!("{word} {title}, {}", format_clock(s.elapsed_secs)),
        false,
    )
    .fill(k.alpha(k.c.danger.icon, tint))
    .on_press(|ui: &mut Ui| ui.select_live())
    .child(icon)
    .child(two_lines(
        [
            grow(
                text(title)
                    .font(k.ty.body_medium)
                    .color(k.c.text_base)
                    .truncate(),
            ),
            clock,
        ],
        [
            grow(status),
            text(mode).font(k.ty.small).color(k.c.text_muted).truncate(),
        ],
    ))
}
