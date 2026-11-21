use greyscreen_engine::RecordingEntry;
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Placement, popover, scrim};

use crate::ui::recordings_filter::BIN;
use crate::ui::{Ui, format_size};

const GAP: f32 = 8.0;
const POPOVER_MIN_WIDTH: f32 = 260.0;
const POPOVER_MAX_WIDTH: f32 = 320.0;

pub(super) fn delete_key(id: &str) -> String {
    format!("delete-{id}")
}

pub(super) fn confirm_delete(rec: &RecordingEntry, k: &Kit, layer: &mut Vec<El>) {
    let question = format!(
        "Delete this {} {} recording?",
        format_size(rec.size_bytes),
        rec.meta.game.info.me.champion
    );
    let buttons = row()
        .w_full()
        .gap(GAP)
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
    let body = col()
        .key("delete-popover")
        .role("dialog")
        .label(question.clone())
        .min_w(POPOVER_MIN_WIDTH)
        .max_w(POPOVER_MAX_WIDTH)
        .padding(12.0)
        .gap(12.0)
        .child(
            col()
                .gap(4.0)
                .child(
                    text(question)
                        .font(k.ty.body_medium)
                        .color(k.c.text_strong)
                        .wrap(),
                )
                .child(
                    text(format!("The game and its clips move to the {BIN}."))
                        .font(k.ty.body)
                        .color(k.c.text_muted)
                        .wrap(),
                ),
        )
        .child(buttons)
        .modal(true, |ui: &mut Ui| ui.cancel_delete());
    layer.push(scrim("delete-scrim", |ui: &mut Ui| ui.cancel_delete()));
    let surface = popover::raised(k, body, k.radius.md);
    layer.push(popover::anchored(
        k,
        surface,
        &delete_key(&rec.meta.id),
        Placement::BottomEnd,
        GAP,
    ));
}
