use std::path::{Component, MAIN_SEPARATOR_STR, Path};

use greyscreen_core::disk::StorageUsage;
use rune::{Cursor, El, col, el, row, text};
use rune_kit::{Kit, ProgressCircle};

use crate::ui::{SettingsSection, Ui, format_size};
use crate::view::layout::{TABULAR, hairline};
use crate::view::widgets::game_icons::{FOLDER_REVEAL, SETTINGS_GEAR};

const WIDTH: f32 = 300.0;
const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 14.0;
const RING: f32 = 40.0;
const BAR: f32 = 4.0;
const ACTION_HEIGHT: f32 = 32.0;
const ACTION_INSET: f32 = 8.0;

pub(super) fn whole(bytes: u64) -> String {
    format_size(bytes).replace(".0 GB", " GB")
}

fn scale(s: &Ui) -> u64 {
    match (s.settings.storage_limit_bytes(), s.storage) {
        (Some(limit), _) => limit,
        (None, Some(usage)) => usage.used(),
        (None, None) => 0,
    }
}

pub(super) fn limited(s: &Ui) -> bool {
    s.settings.storage_limit_bytes().is_some()
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
    names[start..].join(MAIN_SEPARATOR_STR)
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
    let mut details = col()
        .px(PAD_X)
        .pt(PAD_Y)
        .gap(12.0)
        .child(header(s, k))
        .child(summary(s, usage, k))
        .child(breakdown(s, usage, k));
    if let (true, Some(free)) = (limited(s), usage.free) {
        details = details.child(
            text(free_label(&s.settings.output_dir, free))
                .key("storage-free")
                .font(k.ty.text_xsmall)
                .color(k.c.text_muted),
        );
    }
    col()
        .key("storage-popover")
        .w(WIDTH)
        .gap(8.0)
        .child(details)
        .child(el().w_full().px(PAD_X).child(hairline(k)))
        .child(actions(k))
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
    let free = usage
        .free
        .map(|free| format!("{} free on disk", whole(free)));
    let (headline, note) = match s.settings.storage_limit_bytes() {
        Some(limit) => (
            format!("{} of {}", whole(usage.used()), whole(limit)),
            "Storage limit · oldest videos deleted first".to_owned(),
        ),
        None => (
            format!("{} used", whole(usage.used())),
            free.map_or_else(
                || "No storage limit".to_owned(),
                |free| format!("No storage limit · {free}"),
            ),
        ),
    };
    let texts = col()
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
        );
    let ring = limited(s).then(|| {
        ProgressCircle::new(fraction(s))
            .size(RING)
            .label("Storage used")
            .build(k)
    });
    row().gap(12.0).center_y().children(ring).child(texts)
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
        .fill(k.c.layer_02)
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

fn action(k: &Kit, key: &str, label: &str, icon: &'static rune::Icon, on_press: fn(&mut Ui)) -> El {
    row()
        .key(key.to_owned())
        .role("button")
        .label(label.to_owned())
        .w_full()
        .h(ACTION_HEIGHT)
        .px(ACTION_INSET)
        .gap(8.0)
        .center_y()
        .radius(k.radius.lg)
        .cursor(Cursor::Pointer)
        .hover(k.c.hover)
        .on_press(on_press)
        .child(rune::icon(icon, 16.0).color(k.c.icon_base).a11y_hidden())
        .child(text(label.to_owned()).font(k.ty.ui).color(k.c.text_base))
}

fn actions(k: &Kit) -> El {
    col()
        .px(PAD_X - ACTION_INSET)
        .pb(8.0)
        .child(action(
            k,
            "storage-settings",
            "Storage settings…",
            &SETTINGS_GEAR,
            |ui| {
                ui.storage_popover(false);
                ui.open_settings(SettingsSection::Storage);
            },
        ))
        .child(action(
            k,
            "storage-folder",
            "Open recordings folder",
            &FOLDER_REVEAL,
            |ui| {
                ui.storage_popover(false);
                ui.reveal_output_dir();
            },
        ))
}
