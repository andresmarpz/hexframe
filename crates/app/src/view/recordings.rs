mod states;
mod toolbar;

use std::borrow::Cow;
use std::rc::Rc;

use greyscreen_core::game::GameResult;
use greyscreen_engine::{RecordingEntry, reveal};
use jiff::Zoned;
use rune::{Color, Cursor, El, GrowthSpec, Icon, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, glyphs};

use self::states::{confirm_delete, delete_key, empty, no_match};
use self::toolbar::toolbar;
use super::champion::champion_icon;
use super::{card, hairline, page_header};
use crate::ui::library_format::{Kda, kda, mode_label, summary, when};
use crate::ui::{Ui, format_clock, format_size};

const ROW_HEIGHT: f32 = 64.0;
const ICON_SIZE: f32 = 40.0;
const ACCENT_WIDTH: f32 = 3.0;
const COL_RESULT: f32 = 60.0;
const COL_KDA: f32 = 72.0;
const COL_WHEN: f32 = 112.0;
const COL_DURATION: f32 = 58.0;
const COL_CLIPS: f32 = 44.0;
const COL_SIZE: f32 = 62.0;
const COL_ACTION: f32 = 68.0;
const COLUMN_GAP: f32 = 12.0;
const TABULAR: &str = "'tnum' 1";

static TRASH: Icon = Icon {
    name: "greyscreen-trash",
    view: 16.0,
    body: concat!(
        r#"<path d="M2.5 4.5h11M6 4.5V3a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v1.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
        r#"<path d="M4 4.5l.6 8a1.5 1.5 0 0 0 1.5 1.4h3.8a1.5 1.5 0 0 0 1.5-1.4l.6-8M6.8 7v4.5M9.2 7v4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};

static CLIP: Icon = Icon {
    name: "greyscreen-clip",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.5" y="3.5" width="9" height="9" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/>"#,
        r#"<path d="M10.5 7 14.5 4.8v6.4L10.5 9z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>"#
    ),
};

pub(super) fn page(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let open = Button::new("Open folder")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.reveal_output_dir())
        .build(k)
        .key("open-folder");
    let folder = s.settings.output_dir.display().to_string();
    let shown = s.filter.apply(&s.recordings);
    let mut page = col()
        .key("recordings")
        .w_full()
        .h_full()
        .min_h(0.0)
        .gap(12.0)
        .child(page_header(k, "Recordings", &folder, Some(open)));
    if !s.recordings.is_empty() {
        page = page
            .child(
                text(summary(&shown, s.recordings.len()))
                    .key("recordings-summary")
                    .font(k.ty.ui.features(TABULAR))
                    .color(k.c.text_muted),
            )
            .child(toolbar(s, k, layer));
    }
    if let Some(rec) = s.pending_delete_entry() {
        confirm_delete(rec, k, layer);
    }
    let rows: Rc<[usize]> = s
        .recordings
        .iter()
        .enumerate()
        .filter(|(_, rec)| s.filter.accepts(rec))
        .map(|(i, _)| i)
        .collect();
    page.child(table(s, k, rows))
}

fn table(s: &Ui, k: &Kit, rows: Rc<[usize]>) -> El {
    let card = card(k)
        .key("recordings-card")
        .grow(1.0)
        .min_h(0.0)
        .clip_rounded(super::CARD_RADIUS);
    if s.recordings.is_empty() {
        return card.child(empty(k));
    }
    if rows.is_empty() && s.filter.is_active() {
        return card.child(no_match(k));
    }
    let kit = k.clone();
    let count = rows.len();
    let key_rows = rows.clone();
    card.child(header_row(k)).child(hairline(k)).child(
        col()
            .key("recordings-scroll")
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(col().w_full().growth(GrowthSpec::default()).virtual_rows(
                count,
                ROW_HEIGHT,
                move |s: &Ui, i| Cow::Borrowed(s.recordings[key_rows[i]].meta.id.as_str()),
                move |s: &Ui, i| {
                    let rec = &s.recordings[rows[i]];
                    let asking = s.filter.pending_delete.as_deref() == Some(rec.meta.id.as_str());
                    rec_row(rec, asking, &kit)
                },
            )),
    )
}

fn left(width: f32, child: El) -> El {
    row().w(width).center_y().child(child)
}

fn right(width: f32, child: El) -> El {
    row().w(width).center_y().justify_end().child(child)
}

fn header_row(k: &Kit) -> El {
    let head = |label: &str| {
        text(label.to_owned())
            .font(k.ty.small_medium.features(TABULAR))
            .color(k.c.text_muted)
    };
    row()
        .key("recordings-header")
        .w_full()
        .h(34.0)
        .pr(16.0)
        .gap(COLUMN_GAP)
        .center_y()
        .child(el().w(ACCENT_WIDTH))
        .child(el().w(ICON_SIZE))
        .child(el().grow(1.0).min_w(0.0).child(head("Champion")))
        .child(left(COL_RESULT, head("Result")))
        .child(right(COL_KDA, head("KDA")))
        .child(left(COL_WHEN, head("When")))
        .child(right(COL_DURATION, head("Duration")))
        .child(right(COL_CLIPS, head("Clips")))
        .child(right(COL_SIZE, head("Size")))
        .child(el().w(COL_ACTION))
}

fn accent(rec: &RecordingEntry, k: &Kit) -> El {
    let color = match rec.meta.game.result {
        GameResult::Win => k.c.success.icon,
        GameResult::Loss => k.c.danger.icon,
        GameResult::Unknown => k.c.border_strong,
    };
    el().w(ACCENT_WIDTH)
        .h(ROW_HEIGHT - 20.0)
        .radius(ACCENT_WIDTH / 2.0)
        .fill(color)
        .a11y_hidden()
}

fn result_cell(result: GameResult, k: &Kit) -> El {
    let (label, color) = match result {
        GameResult::Win => ("Victory", k.c.success.fg),
        GameResult::Loss => ("Defeat", k.c.danger.fg),
        GameResult::Unknown => ("—", k.c.text_muted),
    };
    text(label).font(k.ty.ui.features(TABULAR)).color(color)
}

fn kda_cell(kda: Kda, k: &Kit) -> El {
    let font = k.ty.ui.features(TABULAR);
    let figure = |n: u32, color: Color| text(n.to_string()).font(font).color(color);
    let slash = || text(" / ").font(font).color(k.c.text_faint);
    row()
        .child(figure(kda.kills, k.c.text_base))
        .child(slash())
        .child(figure(kda.deaths, k.c.danger.fg))
        .child(slash())
        .child(figure(kda.assists, k.c.text_base))
}

fn when_cell(rec: &RecordingEntry, k: &Kit) -> El {
    let font = k.ty.ui.features(TABULAR);
    let shown = when(&rec.meta.started_at, &Zoned::now());
    row()
        .gap(5.0)
        .child(text(shown.day).font(font).color(k.c.text_base))
        .child(text(shown.time).font(font).color(k.c.text_muted))
}

fn clips_cell(count: usize, k: &Kit) -> El {
    if count == 0 {
        return el();
    }
    row()
        .gap(4.0)
        .center_y()
        .child(rune::icon(&CLIP, 13.0).color(k.c.icon_muted).a11y_hidden())
        .child(
            text(count.to_string())
                .font(k.ty.ui.features(TABULAR))
                .color(k.c.text_muted),
        )
}

fn show_button(rec: &RecordingEntry, k: &Kit) -> El {
    let video = rec.folder.join(&rec.meta.video);
    IconButton::new(&glyphs::FOLDER)
        .label("Show in folder")
        .help("Show in folder")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(move |_: &mut Ui| reveal(&video))
        .build(k)
        .key(format!("show-{}", rec.meta.id))
        .group_opacity(0.0, 1.0)
}

fn delete_button(rec: &RecordingEntry, asking: bool, k: &Kit) -> El {
    let id = rec.meta.id.clone();
    let button = IconButton::new(&TRASH)
        .label("Delete recording")
        .help("Delete recording")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(move |ui: &mut Ui| ui.ask_delete(id.clone()))
        .build(k)
        .key(delete_key(&rec.meta.id));
    if asking {
        button
    } else {
        button.group_opacity(0.0, 1.0)
    }
}

fn actions(rec: &RecordingEntry, asking: bool, k: &Kit) -> El {
    row()
        .gap(4.0)
        .child(show_button(rec, k))
        .child(delete_button(rec, asking, k))
}

fn rec_row(rec: &RecordingEntry, asking: bool, k: &Kit) -> El {
    let game = &rec.meta.game;
    let champion = game.info.me.champion.as_str();
    let id = rec.meta.id.clone();
    let font = k.ty.ui.features(TABULAR);
    row()
        .key(rec.meta.id.clone())
        .role("button")
        .label(format!("Review {champion} game"))
        .w_full()
        .h(ROW_HEIGHT)
        .pr(16.0)
        .gap(COLUMN_GAP)
        .center_y()
        .hover(k.c.hover)
        .hover_group()
        .cursor(Cursor::Pointer)
        .on_press(move |ui: &mut Ui| ui.open_review(id.clone()))
        .child(accent(rec, k))
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
        .child(left(COL_RESULT, result_cell(game.result, k)))
        .child(right(
            COL_KDA,
            kda_cell(kda(&rec.meta.events, &game.info.me), k),
        ))
        .child(left(COL_WHEN, when_cell(rec, k)))
        .child(right(
            COL_DURATION,
            text(format_clock(rec.meta.duration_secs as u32))
                .font(font)
                .color(k.c.text_base),
        ))
        .child(right(COL_CLIPS, clips_cell(rec.meta.clips.len(), k)))
        .child(right(
            COL_SIZE,
            text(format_size(rec.size_bytes))
                .font(font)
                .color(k.c.text_muted),
        ))
        .child(right(COL_ACTION, actions(rec, asking, k)))
}
