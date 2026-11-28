use rune::{Cx, El, col, el, row, text};
use rune_kit::{Badge, ControlSize, IconButton, IconButtonVariant, Kit, SegmentedControl};

use super::float::floating;
use crate::logo::LOGO;
use crate::ui::{Screen, Ui};
use crate::view::widgets::game_icons::SETTINGS_GEAR;

pub(crate) const HEIGHT: f32 = 44.0;
const EDGE_PAD: f32 = 16.0;
const LIGHTS_GAP: f32 = 12.0;
const CAPTION_GAP: f32 = 8.0;
const BRAND_WIDTH: f32 = 200.0;
const LOGO_SIZE: f32 = 16.0;
const TRAY_RADIUS: f32 = 10.0;
const TRAY_PAD: f32 = 4.0;
const TABS_WIDTH: f32 = 264.0;
const TABS: [(Screen, &str); 3] = [
    (Screen::Home, "Home"),
    (Screen::Games, "Games"),
    (Screen::Clips, "Clips"),
];

pub(in crate::view) fn top_bar(s: &Ui, k: &Kit, cx: &Cx<Ui>) -> El {
    let insets = cx.chrome_insets();
    let start = if insets.leading > 0.0 {
        insets.leading + LIGHTS_GAP
    } else {
        EDGE_PAD
    };
    el().key("top-bar")
        .pin(Some(0.0), Some(0.0), Some(0.0), None)
        .h(HEIGHT)
        .child(
            row()
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .pl(start)
                .pr(insets.trailing + CAPTION_GAP)
                .center_y()
                .window_drag()
                .child(brand(k))
                .child(row().grow(1.0).min_w(0.0).h_full().window_drag())
                .child(settings_button(k)),
        )
        .child(
            row()
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .center_y()
                .justify_center()
                .child(top_dock(s, k)),
        )
}

fn brand(k: &Kit) -> El {
    row()
        .key("brand")
        .w(BRAND_WIDTH)
        .gap(8.0)
        .center_y()
        .window_drag()
        .child(
            rune::icon(&LOGO, LOGO_SIZE)
                .color(k.c.text_base)
                .a11y_hidden(),
        )
        .child(text("Greyscreen").font(k.ty.ui_medium).color(k.c.text_base))
}

fn top_dock(s: &Ui, k: &Kit) -> El {
    let labels: Vec<&str> = TABS.iter().map(|(_, label)| *label).collect();
    let selected = (!s.settings_open)
        .then(|| TABS.iter().position(|(screen, _)| *screen == s.screen))
        .flatten();
    let mut tabs = SegmentedControl::new(&labels, selected)
        .label("Navigation")
        .width(Some(TABS_WIDTH))
        .on_pick(|ui: &mut Ui, i: usize, _: &mut rune::Tx| ui.go(TABS[i].0));
    let clips = s.clip_count();
    if clips > 0 {
        tabs = tabs.trailing(2, Badge::new(clips.to_string()).build(k));
    }
    tray(k).child(tabs.build(k).key("top-tabs"))
}

fn tray(k: &Kit) -> El {
    floating(
        col().key("top-dock").padding(TRAY_PAD).radius(TRAY_RADIUS),
        k,
    )
}

fn settings_button(k: &Kit) -> El {
    IconButton::new(&SETTINGS_GEAR)
        .label("Settings")
        .help("Settings")
        .variant(IconButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.toggle_settings())
        .build(k)
        .key("top-settings")
}
