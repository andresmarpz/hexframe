use std::borrow::Cow;

use greyscreen_core::game::GameResult;
use greyscreen_engine::{RecordingEntry, reveal};
use rune::{El, GrowthSpec, ScrollSpec, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Button, ButtonVariant, ControlSize, Kit};

use super::{card, hairline, page_header};
use crate::ui::{Ui, format_clock, format_date, format_size};

const ROW_HEIGHT: f32 = 52.0;
const COL_DATE: f32 = 120.0;
const COL_RESULT: f32 = 60.0;
const COL_DURATION: f32 = 62.0;
const COL_SIZE: f32 = 64.0;
const COL_CLIPS: f32 = 44.0;
const COL_ACTIONS: f32 = 132.0;
const EMPTY_TEXT: &str = "No recordings yet. Play a game, or run a test recording from Status.";

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let subtitle = format!(
        "{} recordings in {}",
        s.recordings.len(),
        s.settings.output_dir.display()
    );
    let open = Button::new("Open folder")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.reveal_output_dir())
        .build(k)
        .key("open-folder");
    col()
        .key("recordings")
        .w_full()
        .h_full()
        .min_h(0.0)
        .gap(16.0)
        .child(page_header(k, "Recordings", &subtitle, Some(open)))
        .child(table(s, k))
}

fn table(s: &Ui, k: &Kit) -> El {
    let body = if s.recordings.is_empty() {
        col()
            .key("recordings-empty")
            .w_full()
            .grow(1.0)
            .center()
            .px(24.0)
            .child(text(EMPTY_TEXT).font(k.ty.body).color(k.c.text_muted))
    } else {
        let kit = k.clone();
        col()
            .key("recordings-scroll")
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(col().w_full().growth(GrowthSpec::default()).virtual_rows(
                s.recordings.len(),
                ROW_HEIGHT,
                |s: &Ui, i| Cow::Borrowed(s.recordings[i].meta.id.as_str()),
                move |s: &Ui, i| rec_row(&s.recordings[i], &kit),
            ))
    };
    card(k)
        .key("recordings-card")
        .grow(1.0)
        .min_h(0.0)
        .child(header_row(k))
        .child(hairline(k))
        .child(body)
}

fn cell(width: f32, child: El) -> El {
    el().w(width).child(child)
}

fn right_cell(width: f32, child: El) -> El {
    row().w(width).justify_end().child(child)
}

fn header_row(k: &Kit) -> El {
    let head = |label: &str| {
        text(label.to_owned())
            .font(k.ty.small_medium)
            .color(k.c.text_muted)
    };
    row()
        .w_full()
        .h(36.0)
        .px(16.0)
        .gap(10.0)
        .center_y()
        .child(cell(COL_DATE, head("Date")))
        .child(el().grow(1.0).min_w(0.0).child(head("Champion")))
        .child(cell(COL_RESULT, head("Result")))
        .child(right_cell(COL_DURATION, head("Duration")))
        .child(right_cell(COL_SIZE, head("Size")))
        .child(right_cell(COL_CLIPS, head("Clips")))
        .child(el().w(COL_ACTIONS))
}

fn actions(rec: &RecordingEntry, k: &Kit) -> El {
    let id = rec.meta.id.clone();
    let video = rec.folder.join(&rec.meta.video);
    let show = Button::new("Show")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(move |_: &mut Ui| reveal(&video))
        .build(k)
        .key(format!("show-{id}"));
    let review = Button::new("Review")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .on_press({
            let id = id.clone();
            move |ui: &mut Ui| ui.open_review(id.clone())
        })
        .build(k)
        .key(format!("review-{id}"));
    row()
        .w(COL_ACTIONS)
        .gap(6.0)
        .justify_end()
        .child(show)
        .child(review)
}

fn rec_row(rec: &RecordingEntry, k: &Kit) -> El {
    let muted = |label: String| text(label).font(k.ty.body).color(k.c.text_muted);
    let game = &rec.meta.game;
    let result = match game.result {
        GameResult::Win => Badge::new("WIN").variant(BadgeVariant::Accent).build(k),
        GameResult::Loss => Badge::new("LOSS").variant(BadgeVariant::Subtle).build(k),
        GameResult::Unknown => muted("-".into()),
    };
    row()
        .key(rec.meta.id.clone())
        .w_full()
        .h(ROW_HEIGHT)
        .px(16.0)
        .gap(10.0)
        .center_y()
        .child(cell(COL_DATE, muted(format_date(&rec.meta.started_at))))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(
                    text(game.info.me.champion.clone())
                        .font(k.ty.body_medium)
                        .color(k.c.text_base)
                        .truncate(),
                )
                .child(
                    text(game.info.mode.clone())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .truncate(),
                ),
        )
        .child(cell(COL_RESULT, result))
        .child(right_cell(
            COL_DURATION,
            text(format_clock(rec.meta.duration_secs as u32))
                .font(k.ty.body)
                .color(k.c.text_base),
        ))
        .child(right_cell(COL_SIZE, muted(format_size(rec.size_bytes))))
        .child(right_cell(
            COL_CLIPS,
            muted(rec.meta.clips.len().to_string()),
        ))
        .child(actions(rec, k))
}
