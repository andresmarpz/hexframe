mod face;
mod storage;

use greyscreen_core::keybinds::Action;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{Cursor, El, Shadow, col, el, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, Keybind, Kit, Placement, ProgressCircle, popover, scrim,
};

use super::bottom::DOCK_HEIGHT;
use super::float::floating;
use crate::ui::{MAC, Ui};
use crate::view::layout::TABULAR;
use face::{Dot, Face};

const DOCK_PAD: f32 = 10.0;
const GAP: f32 = 12.0;
const ITEM: f32 = 28.0;
const DIVIDER_AIR: f32 = 4.0;
const DIVIDER_HEIGHT: f32 = 20.0;
const DOT: f32 = 10.0;
const GLOW: f32 = 3.0;
const POPOVER_GAP: f32 = 8.0;
const POPOVER_RADIUS: f32 = 12.0;
const KEY: &str = "recorder-dock";

pub(super) fn dock(s: &Ui, k: &Kit) -> El {
    let surface = row()
        .key(KEY)
        .h(DOCK_HEIGHT)
        .px(DOCK_PAD)
        .gap(GAP)
        .center_y()
        .radius(DOCK_HEIGHT / 2.0)
        .child(status(s, k))
        .child(divider(k))
        .child(save_clip(s, k))
        .child(divider(k))
        .child(storage_button(s, k, s.dock.storage_open));
    floating(surface, k)
}

pub(super) fn overlay(s: &Ui, k: &Kit, layer: &mut Vec<El>) {
    if !s.dock.storage_open {
        return;
    }
    layer.push(scrim("storage-scrim", |ui: &mut Ui| {
        ui.storage_popover(false)
    }));
    let surface = floating(storage::body(s, k).radius(POPOVER_RADIUS), k)
        .role("dialog")
        .label("Storage")
        .focus_return("dock-storage");
    layer.push(popover::anchored(
        k,
        surface,
        KEY,
        Placement::TopEnd,
        POPOVER_GAP,
    ));
}

fn status(s: &Ui, k: &Kit) -> El {
    let face = Face::of(s, k);
    row()
        .gap(GAP)
        .center_y()
        .child(record_button(s, &face, k))
        .child(labels(&face, k))
}

fn record_button(s: &Ui, face: &Face, k: &Kit) -> El {
    let armed = s.settings.auto_record;
    let help = if armed {
        "Pause recording"
    } else {
        "Record games"
    };
    let shadow = |y: f32, blur: f32, spread: f32, color, inset| Shadow {
        x: 0.0,
        y,
        blur,
        spread,
        color,
        inset,
    };
    let mut disc = el()
        .key("record-button")
        .role("button")
        .label(help)
        .help(help)
        .size(ITEM, ITEM)
        .radius(ITEM / 2.0)
        .fill(k.c.bg_base)
        .linear_gradient(
            std::f32::consts::PI,
            [(0.0, k.light(0.10)), (1.0, k.light(0.0))],
        )
        .center()
        .cursor(Cursor::Pointer)
        .on_press(|ui: &mut Ui| ui.change(|s| s.auto_record = !s.auto_record))
        .box_shadow(shadow(0.5, 0.0, 0.0, k.light(0.12), true));
    if let Some(glow) = face.glow {
        disc = disc.box_shadow(shadow(0.0, 0.0, GLOW, k.alpha(glow, 0.25), false));
    }
    disc.box_shadow(shadow(1.0, 2.0, 0.0, k.dark_a(0.3), false))
        .box_shadow(shadow(0.0, 0.0, 0.5, k.light(0.16), false))
        .child(dot(face.dot))
}

fn dot(dot: Dot) -> El {
    let base = el().size(DOT, DOT).radius(DOT / 2.0).a11y_hidden();
    match dot {
        Dot::Solid(color) => base.fill(color),
        Dot::Hollow(color) => base.border(1.5, color),
    }
}

fn labels(face: &Face, k: &Kit) -> El {
    let mut subtitle = text(face.subtitle.clone())
        .key("dock-subtitle")
        .font(k.ty.small.lh(13.0))
        .color(k.c.text_muted);
    if face.tabular {
        subtitle = subtitle.font(k.ty.small.lh(13.0).features(TABULAR));
    }
    col()
        .key("dock-status")
        .gap(1.0)
        .child(
            text(face.title.clone())
                .key("dock-title")
                .font(k.ty.ui_medium.lh(16.0))
                .color(face.title_color),
        )
        .child(subtitle)
}

fn divider(k: &Kit) -> El {
    row().px(DIVIDER_AIR).child(
        el().w(1.0)
            .h(DIVIDER_HEIGHT)
            .fill(k.c.border_base)
            .a11y_hidden(),
    )
}

fn save_clip(s: &Ui, k: &Kit) -> El {
    let recording = matches!(s.status, EngineStatus::Recording { .. });
    let keys = s
        .settings
        .keymap()
        .bindings(Action::SaveClip)
        .first()
        .map(|combo| combo.keycaps(MAC));
    let mut button = Button::new("Save clip")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .disabled(!recording)
        .on_press(|ui: &mut Ui| ui.send(EngineCmd::SaveClipNow));
    if recording {
        button = button.help("Save a clip of the last moments");
    }
    let keybind = keys.map(|keys| Keybind::new(&keys).build(k));
    button
        .label_with(move |font, ink| {
            row()
                .gap(8.0)
                .center_y()
                .child(text("Save clip").font(font).color(ink))
                .children(keybind)
        })
        .build(k)
        .key("dock-save-clip")
}

fn storage_button(s: &Ui, k: &Kit, open: bool) -> El {
    let label = s
        .storage
        .map_or_else(|| "Storage".to_owned(), |u| storage::whole(u.used()));
    let ring = storage::limited(s).then(|| {
        ProgressCircle::new(storage::fraction(s))
            .size(16.0)
            .build(k)
    });
    Button::new("Storage")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .expanded(open)
        .label_with(move |font, ink| {
            row()
                .gap(6.0)
                .center_y()
                .children(ring)
                .child(text(label).font(font.features(TABULAR)).color(ink))
        })
        .on_press(move |ui: &mut Ui| ui.storage_popover(!open))
        .build(k)
        .key("dock-storage")
}
