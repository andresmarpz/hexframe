use rune::{Cursor, El, Side, TextStyle, col, row, text};
use rune_kit::{Kit, Progress, ProgressCircle};

use crate::ui::{LibraryTotals, SettingsSection, Ui, format_size};

const RING_SIZE: f32 = 22.0;

fn limit_label(s: &Ui) -> Option<String> {
    s.settings.storage_limit_gb.map(|gb| format!("{gb} GB"))
}

fn description(s: &Ui, totals: &LibraryTotals) -> String {
    let used = format_size(totals.bytes);
    match limit_label(s) {
        Some(limit) => format!("Storage: {used} of {limit} used"),
        None => format!("Storage: {used} used"),
    }
}

pub(super) fn block(s: &Ui, k: &Kit) -> El {
    let totals = LibraryTotals::of(&s.recordings);
    let figures = TextStyle {
        features: "'tnum' 1",
        ..k.ty.small
    };
    let used = format_size(totals.bytes);
    let amount = match limit_label(s) {
        Some(limit) => format!("{used} of {limit}"),
        None => used,
    };
    let mut block = col()
        .key("nav-storage")
        .role("button")
        .label(description(s, &totals))
        .help("Open storage settings")
        .focusable(true)
        .w_full()
        .px(8.0)
        .py(4.0)
        .gap(6.0)
        .radius(k.radius.md)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .on_press(|ui: &mut Ui| ui.open_settings(SettingsSection::Storage))
        .child(
            row()
                .w_full()
                .center_y()
                .justify_between()
                .child(
                    text("Storage")
                        .font(k.ty.small_medium)
                        .color(k.c.text_muted)
                        .a11y_hidden(),
                )
                .child(
                    text(amount)
                        .font(figures)
                        .color(k.c.text_muted)
                        .a11y_hidden(),
                ),
        );
    if let Some(fraction) = totals.used_fraction(s.settings.storage_limit_bytes()) {
        block = block.child(
            Progress::new(Some(fraction))
                .build(k)
                .key("nav-storage-meter"),
        );
    }
    block
}

pub(super) fn ring(s: &Ui, k: &Kit) -> Option<El> {
    let totals = LibraryTotals::of(&s.recordings);
    let used = totals.used_fraction(s.settings.storage_limit_bytes())?;
    let tip = description(s, &totals);
    Some(
        row()
            .key("nav-storage")
            .role("button")
            .label(tip.clone())
            .help(format!("{tip} — click for settings"))
            .help_side(Side::Right)
            .focusable(true)
            .size(40.0, 28.0)
            .center()
            .radius(k.radius.md)
            .hover(k.c.hover)
            .cursor(Cursor::Pointer)
            .on_press(|ui: &mut Ui| ui.open_settings(SettingsSection::Storage))
            .child(ProgressCircle::new(used).size(RING_SIZE).build(k)),
    )
}
