use greyscreen_engine::EngineStatus;
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Switch};

use super::sidebar::icons::{CLIPS, GAMES, GEAR, HOME};
use crate::ui::{Screen, Ui};

const HEIGHT: f32 = 44.0;
const BOTTOM_GAP: f32 = 16.0;
pub(in crate::view) const CLEARANCE: f32 = HEIGHT + BOTTOM_GAP * 2.0;
const PAD: f32 = 4.0;
const RECORD_WIDTH: f32 = 112.0;
const BLUR: f32 = 18.0;
const GLASS_ALPHA: f32 = 0.78;

pub(in crate::view) fn layer(s: &Ui, k: &Kit) -> El {
    row()
        .key("dock-layer")
        .pin(Some(0.0), None, Some(0.0), Some(BOTTOM_GAP))
        .justify_center()
        .child(dock(s, k))
}

fn dock(s: &Ui, k: &Kit) -> El {
    let surface = row()
        .key("dock")
        .role("navigation")
        .label("Navigation")
        .h(HEIGHT)
        .padding(PAD)
        .gap(2.0)
        .center_y()
        .radius(HEIGHT / 2.0)
        .border(1.0, k.c.border_strong)
        .backdrop_blur(BLUR)
        .fill(k.alpha(k.c.bg_base, GLASS_ALPHA))
        .width_leads()
        .child(record_segment(s, k))
        .child(divider(k))
        .child(tab(s, k, Screen::Home, "Home", &HOME, "dock-home"))
        .child(tab(s, k, Screen::Games, "Games", &GAMES, "dock-games"))
        .child(tab(s, k, Screen::Clips, "Clips", &CLIPS, "dock-clips"))
        .child(divider(k))
        .child(settings_button(s, k));
    k.elevate(surface, &k.elevation.floating)
}

fn divider(k: &Kit) -> El {
    el().w(1.0)
        .h(20.0)
        .mx(4.0)
        .fill(k.c.border_muted)
        .a11y_hidden()
}

fn tab(
    s: &Ui,
    k: &Kit,
    screen: Screen,
    label: &'static str,
    icon: &'static rune::Icon,
    key: &'static str,
) -> El {
    let variant = if s.screen == screen && !s.settings_open {
        ButtonVariant::Neutral
    } else {
        ButtonVariant::Ghost
    };
    Button::new(label)
        .icon(icon)
        .variant(variant)
        .control_size(ControlSize::Large)
        .on_press(move |ui: &mut Ui| ui.go(screen))
        .build(k)
        .key(key)
}

fn settings_button(s: &Ui, k: &Kit) -> El {
    let variant = if s.settings_open {
        ButtonVariant::Neutral
    } else {
        ButtonVariant::Ghost
    };
    Button::new("Settings")
        .icon(&GEAR)
        .variant(variant)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.toggle_settings())
        .build(k)
        .key("dock-settings")
}

fn record_segment(s: &Ui, k: &Kit) -> El {
    let on = s.settings.auto_record;
    let (dot, ink) = match (&s.status, on) {
        (EngineStatus::Error(_), _) => (k.c.danger.icon, k.c.danger.fg),
        (EngineStatus::Recording { .. }, _) => (k.c.danger.icon, k.c.text_base),
        (_, true) => (k.c.success.icon, k.c.text_base),
        (_, false) => (k.c.text_faint, k.c.text_muted),
    };
    let label = match &s.status {
        EngineStatus::Finalizing => "Saving",
        EngineStatus::Waiting => "Wait",
        EngineStatus::Error(_) => "Error",
        _ => "Rec",
    };
    row()
        .key("dock-record")
        .w(RECORD_WIDTH)
        .h_full()
        .px(12.0)
        .gap(8.0)
        .center_y()
        .child(el().size(8.0, 8.0).radius(4.0).fill(dot).a11y_hidden())
        .child(text(label).font(k.ty.ui_medium).color(ink))
        .child(col().grow(1.0).a11y_hidden())
        .child(
            Switch::new(on)
                .label("Record games".to_owned())
                .hide_label()
                .control_size(ControlSize::Small)
                .on_toggle(|ui: &mut Ui| ui.change(|s| s.auto_record = !s.auto_record))
                .build(k)
                .key("record-games"),
        )
}
