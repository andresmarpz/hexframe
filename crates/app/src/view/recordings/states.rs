use greyscreen_engine::RecordingEntry;
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Placement, popover, scrim};

use super::CLIP;
use crate::ui::recordings_filter::BIN;
use crate::ui::{Screen, Ui, format_size};

const GAP: f32 = 8.0;
const POPOVER_MIN_WIDTH: f32 = 260.0;
const POPOVER_MAX_WIDTH: f32 = 320.0;

pub(super) fn delete_key(id: &str) -> String {
    format!("delete-{id}")
}

pub(super) fn empty(k: &Kit) -> El {
    col()
        .key("recordings-empty")
        .w_full()
        .grow(1.0)
        .center()
        .gap(16.0)
        .px(24.0)
        .child(rune::icon(&CLIP, 32.0).color(k.c.icon_muted).a11y_hidden())
        .child(
            col()
                .items_center()
                .gap(4.0)
                .child(
                    text("No recordings yet")
                        .font(k.ty.body_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(
                        "Play a League game and Greyscreen records it automatically, \
                         or run a 10-second test from Home.",
                    )
                    .font(k.ty.body)
                    .color(k.c.text_muted),
                ),
        )
        .child(
            Button::new("Go to Home")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .on_press(|ui: &mut Ui| ui.go(Screen::Home))
                .build(k)
                .key("go-to-home"),
        )
}

pub(super) fn no_match(k: &Kit) -> El {
    col()
        .key("recordings-no-match")
        .w_full()
        .grow(1.0)
        .center()
        .gap(16.0)
        .px(24.0)
        .child(
            text("No games match these filters")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            Button::new("Clear filters")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .on_press(|ui: &mut Ui| ui.filter.clear())
                .build(k)
                .key("clear-filters"),
        )
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
