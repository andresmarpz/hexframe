use std::borrow::Cow;
use std::path::Path;
use std::process::Command;
use std::thread;

use rune::{El, GrowthSpec, ScrollSpec, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Button, ButtonVariant, ControlSize, Kit};

use super::{card, hairline, page_header};
use crate::ui::{RecRow, ResultKind, Ui, format_clock, format_size};

const ROW_HEIGHT: f32 = 44.0;
const COL_MODE: f32 = 90.0;
const COL_RESULT: f32 = 76.0;
const COL_DURATION: f32 = 76.0;
const COL_SIZE: f32 = 80.0;
const COL_DATE: f32 = 150.0;

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let subtitle = format!(
        "{} recordings in {}",
        s.recordings.len(),
        s.settings.output_dir.display()
    );
    let open = Button::new("Open folder")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| open_folder(&ui.settings.output_dir))
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
        col().w_full().grow(1.0).center().child(
            text("No recordings yet")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
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
                |s: &Ui, i| Cow::Owned(s.recordings[i].id.clone()),
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
        .gap(12.0)
        .center_y()
        .child(cell(COL_DATE, head("Date")))
        .child(el().grow(1.0).child(head("Champion")))
        .child(cell(COL_MODE, head("Mode")))
        .child(cell(COL_RESULT, head("Result")))
        .child(right_cell(COL_DURATION, head("Duration")))
        .child(right_cell(COL_SIZE, head("Size")))
}

fn rec_row(rec: &RecRow, k: &Kit) -> El {
    let body = |label: String| text(label).font(k.ty.body).color(k.c.text_base).truncate();
    let muted = |label: String| text(label).font(k.ty.body).color(k.c.text_muted);
    let (result_label, result_variant) = match rec.result {
        ResultKind::Win => ("WIN", BadgeVariant::Accent),
        ResultKind::Loss => ("LOSS", BadgeVariant::Subtle),
        ResultKind::Unknown => ("-", BadgeVariant::Neutral),
    };
    let result = if rec.result == ResultKind::Unknown {
        muted("Unknown".into())
    } else {
        Badge::new(result_label).variant(result_variant).build(k)
    };
    row()
        .key(rec.id.clone())
        .w_full()
        .h(ROW_HEIGHT)
        .px(16.0)
        .gap(12.0)
        .center_y()
        .child(cell(COL_DATE, muted(rec.started.clone())))
        .child(
            el().grow(1.0).child(
                text(rec.champion.clone())
                    .font(k.ty.body_medium)
                    .color(k.c.text_base),
            ),
        )
        .child(cell(COL_MODE, muted(rec.mode.clone())))
        .child(cell(COL_RESULT, result))
        .child(right_cell(
            COL_DURATION,
            body(format_clock(rec.duration_secs)),
        ))
        .child(right_cell(COL_SIZE, muted(format_size(rec.size_bytes))))
}

fn open_folder(dir: &Path) {
    let dir = dir.to_path_buf();
    thread::spawn(move || {
        if let Err(err) = std::fs::create_dir_all(&dir) {
            tracing::warn!(path = %dir.display(), %err, "cannot create output folder");
        }
        let opener = if cfg!(windows) {
            "explorer.exe"
        } else {
            "open"
        };
        if let Err(err) = Command::new(opener).arg(&dir).status() {
            tracing::warn!(path = %dir.display(), %err, "cannot open folder");
        }
    });
}
