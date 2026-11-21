use greyscreen_engine::RecordingEntry;
use jiff::Zoned;
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Dialog, Kit};

use crate::ui::library_format::when;
use crate::ui::recordings_filter::BIN;
use crate::ui::{Ui, format_size};

const WIDTH: f32 = 400.0;
const PADDING: f32 = 16.0;

pub(super) fn delete_key(id: &str) -> String {
    format!("delete-{id}")
}

pub(super) fn confirm_delete(rec: &RecordingEntry, k: &Kit, layer: &mut Vec<El>) {
    let shown = when(&rec.meta.started_at, &Zoned::now());
    let summary = format!(
        "{} · {} {} · {}",
        rec.meta.game.info.me.champion,
        shown.day,
        shown.time,
        format_size(rec.size_bytes)
    );
    let details = col()
        .w_full()
        .gap(4.0)
        .child(
            text(summary)
                .key("delete-summary")
                .font(k.ty.body_medium)
                .color(k.c.text_strong)
                .wrap(),
        )
        .child(
            text(format!("The game and its clips move to the {BIN}."))
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap(),
        );
    let buttons = row()
        .w_full()
        .gap(8.0)
        .justify_end()
        .child(
            Button::new("Cancel")
                .variant(ButtonVariant::Neutral)
                .on_press(|ui: &mut Ui| ui.cancel_delete())
                .build(k)
                .key("delete-cancel"),
        )
        .child(
            Button::new("Delete")
                .variant(ButtonVariant::Danger)
                .on_press(|ui: &mut Ui| ui.confirm_delete())
                .build(k)
                .key("delete-confirm"),
        );
    Dialog::new("delete", "Delete this game?")
        .width(WIDTH)
        .fit()
        .open(true)
        .on_close(|ui: &mut Ui| ui.cancel_delete())
        .child(
            col()
                .w_full()
                .px(PADDING)
                .pb(PADDING)
                .gap(PADDING)
                .child(details)
                .child(buttons),
        )
        .build(k, layer);
}
