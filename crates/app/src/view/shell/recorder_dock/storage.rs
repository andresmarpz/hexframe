use std::path::{Component, MAIN_SEPARATOR, Path};

use greyscreen_core::disk::StorageUsage;
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, ProgressCircle};

use crate::ui::{SettingsSection, Ui, format_size};
use crate::view::layout::{TABULAR, hairline};
use crate::view::widgets::game_icons::{FOLDER_REVEAL, SETTINGS_GEAR};

const WIDTH: f32 = 300.0;
const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 14.0;
const RING: f32 = 40.0;
const BAR: f32 = 4.0;

pub(super) fn whole(bytes: u64) -> String {
    format_size(bytes).replace(".0 GB", " GB")
}

fn scale(s: &Ui) -> u64 {
    match (s.settings.storage_limit_bytes(), s.storage) {
        (Some(limit), _) => limit,
        (None, Some(usage)) => usage.used() + usage.free.unwrap_or(0),
        (None, None) => 0,
    }
}

pub(super) fn fraction(s: &Ui) -> f32 {
    let used = s.storage.map_or(0, |usage| usage.used());
    match scale(s) {
        0 => 0.0,
        total => (used as f32 / total as f32).clamp(0.0, 1.0),
    }
}

fn short_path(dir: &Path) -> String {
    let names: Vec<String> = dir
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let start = names.len().saturating_sub(2);
    names[start..].join(&MAIN_SEPARATOR.to_string())
}

fn free_label(dir: &Path, free: u64) -> String {
    let drive = dir.components().find_map(|part| match part {
        Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
        _ => None,
    });
    match drive {
        Some(drive) => format!("Free on drive {drive} {}", whole(free)),
        None => format!("Free on disk: {}", whole(free)),
    }
}

pub(super) fn body(s: &Ui, k: &Kit) -> El {
    let usage = s.storage.unwrap_or_default();
    let mut body = col()
        .key("storage-popover")
        .w(WIDTH)
        .px(PAD_X)
        .py(PAD_Y)
        .gap(12.0)
        .child(header(s, k))
        .child(summary(s, usage, k))
        .child(breakdown(s, usage, k));
    if let Some(free) = usage.free {
        body = body.child(
            text(free_label(&s.settings.output_dir, free))
                .key("storage-free")
                .font(k.ty.text_xsmall)
                .color(k.c.text_muted),
        );
    }
    body.child(hairline(k)).child(actions(k))
}

fn header(s: &Ui, k: &Kit) -> El {
    row()
        .justify_between()
        .center_y()
        .gap(8.0)
        .child(text("Storage").font(k.ty.ui_medium).color(k.c.text_base))
        .child(
            text(short_path(&s.settings.output_dir))
                .key("storage-path")
                .font(k.ty.text_xsmall)
                .color(k.c.text_muted)
                .truncate(),
        )
}

fn summary(s: &Ui, usage: StorageUsage, k: &Kit) -> El {
    let (headline, note) = match s.settings.storage_limit_bytes() {
        Some(limit) => (
            format!("{} of {}", whole(usage.used()), whole(limit)),
            "Storage limit · oldest videos deleted first",
        ),
        None => (format!("{} used", whole(usage.used())), "No storage limit"),
    };
    row()
        .gap(12.0)
        .center_y()
        .child(
            ProgressCircle::new(fraction(s))
                .size(RING)
                .label("Storage used")
                .build(k),
        )
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(
                    text(headline)
                        .key("storage-headline")
                        .font(rune::TextStyle {
                            size: 15.0,
                            line_height: 20.0,
                            ..k.ty.ui_medium
                        })
                        .color(k.c.text_base),
                )
                .child(
                    text(note)
                        .key("storage-note")
                        .wrap()
                        .w_full()
                        .font(k.ty.text_xsmall)
                        .color(k.c.text_muted),
                ),
        )
}

fn breakdown(s: &Ui, usage: StorageUsage, k: &Kit) -> El {
    let total = scale(s);
    col()
        .gap(10.0)
        .child(line(k, "Game videos", usage.videos, total, k.c.bg_accent))
        .child(line(k, "Clips", usage.clips, total, k.c.info.fg))
        .child(line(
            k,
            "Thumbnails & timelines",
            usage.previews,
            total,
            k.c.text_muted,
        ))
}

fn line(k: &Kit, label: &str, bytes: u64, total: u64, color: rune::Color) -> El {
    let share = match total {
        0 => 0.0,
        total => (bytes as f32 / total as f32).clamp(0.0, 1.0),
    };
    let bar = row()
        .w_full()
        .h(BAR)
        .radius(BAR / 2.0)
        .fill(k.c.layer_03)
        .a11y_hidden()
        .child(
            el().grow(share.max(0.001))
                .h(BAR)
                .radius(BAR / 2.0)
                .fill(color),
        )
        .child(el().grow((1.0 - share).max(0.001)).h(BAR));
    col()
        .gap(5.0)
        .child(
            row()
                .justify_between()
                .child(text(label).font(k.ty.text_xsmall).color(k.c.text_base))
                .child(
                    text(format_size(bytes))
                        .font(k.ty.text_xsmall.features(TABULAR))
                        .color(k.c.text_muted),
                ),
        )
        .child(bar)
}

fn actions(k: &Kit) -> El {
    let settings = Button::new("Storage settings…")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .icon(&SETTINGS_GEAR)
        .on_press(|ui: &mut Ui| {
            ui.storage_popover(false);
            ui.open_settings(SettingsSection::Storage);
        })
        .build(k)
        .key("storage-settings")
        .w_full()
        .justify_start();
    let folder = Button::new("Open recordings folder")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .icon(&FOLDER_REVEAL)
        .on_press(|ui: &mut Ui| {
            ui.storage_popover(false);
            ui.reveal_output_dir();
        })
        .build(k)
        .key("storage-folder")
        .w_full()
        .justify_start();
    col().gap(2.0).child(settings).child(folder)
}
