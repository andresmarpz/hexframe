use rune::{El, Icon, Side, col, el, row};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit, State};

use super::icons::{CLIPS, FEEDBACK, GAMES, GEAR, HOME, LOGO, SIDEBAR};
use super::{recent, status, storage};
use crate::ui::{Screen, Ui};

pub(super) const WIDTH: f32 = 56.0;
const TOP_BAND: f32 = 14.0;
const TRAFFIC_LIGHT_BAND: f32 = 34.0;
const LOGO_SIZE: f32 = 24.0;
const CELL: f32 = 44.0;
const INDICATOR_WIDTH: f32 = 2.0;
const INDICATOR_HEIGHT: f32 = 20.0;

pub(super) fn panel(s: &Ui, k: &Kit, titlebar_inset: f32) -> El {
    let mut footer = col().key("nav-footer").w_full().pb(8.0).gap(2.0);
    if let Some(ring) = storage::ring(s, k) {
        footer = footer.child(row().w_full().h(30.0).center().child(ring));
    }
    footer = footer.child(cell(
        k,
        rail_button(&FEEDBACK, "Send feedback", s.feedback.open)
            .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.open_feedback(tx))
            .build(k)
            .key("nav-feedback")
            .help_side(Side::Right),
        s.feedback.open,
    ));
    footer = footer.child(cell(
        k,
        rail_button(&GEAR, "Settings", s.settings_open)
            .on_press(|ui: &mut Ui| ui.toggle_settings())
            .build(k)
            .key("nav-settings")
            .help_side(Side::Right),
        s.settings_open,
    ));
    col()
        .key("nav")
        .w(WIDTH)
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_deep)
        .items_center()
        .child(brand(s, k, titlebar_inset))
        .child(
            row()
                .w_full()
                .h(CELL - 4.0)
                .center()
                .child(status::dot_button(s, k)),
        )
        .child(
            col()
                .key("nav-items")
                .role("navigation")
                .label("Navigation")
                .w_full()
                .gap(2.0)
                .child(item(k, Screen::Home, "Home", &HOME, "nav-home", s))
                .child(item(k, Screen::Games, "Games", &GAMES, "nav-games", s))
                .child(item(k, Screen::Clips, "Clips", &CLIPS, "nav-clips", s)),
        )
        .child(divider(k))
        .child(
            col()
                .key("nav-recent-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(recent::rail(s, k)),
        )
        .child(footer)
}

fn brand(s: &Ui, k: &Kit, titlebar_inset: f32) -> El {
    let band = if titlebar_inset > 0.0 {
        TRAFFIC_LIGHT_BAND
    } else {
        TOP_BAND
    };
    let expand = IconButton::new(&SIDEBAR)
        .label("Expand sidebar")
        .help(format!("Expand sidebar ({})", s.nav_shortcut()))
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.toggle_nav())
        .build(k)
        .key("nav-collapse")
        .help_side(Side::Right);
    col()
        .key("nav-top")
        .w_full()
        .items_center()
        .window_drag()
        .child(el().w_full().h(band).a11y_hidden())
        .child(
            row().key("nav-brand").h(36.0).center().child(
                rune::icon(&LOGO, LOGO_SIZE)
                    .color(k.c.bg_accent)
                    .a11y_hidden(),
            ),
        )
        .child(row().w_full().h(CELL - 4.0).center().child(expand))
}

fn divider(k: &Kit) -> El {
    row()
        .w_full()
        .h(9.0)
        .center()
        .child(el().w(24.0).h(1.0).fill(k.c.border_muted).a11y_hidden())
}

fn item(
    k: &Kit,
    screen: Screen,
    label: &'static str,
    icon: &'static Icon,
    key: &'static str,
    s: &Ui,
) -> El {
    let active = s.screen == screen;
    cell(
        k,
        rail_button(icon, label, active)
            .on_press(move |ui: &mut Ui| ui.go(screen))
            .build(k)
            .key(key)
            .help_side(Side::Right),
        active,
    )
}

fn cell(k: &Kit, button: El, active: bool) -> El {
    let cell = row().w_full().h(CELL).center().child(button);
    if active {
        cell.child(indicator(k))
    } else {
        cell
    }
}

fn rail_button(icon: &'static Icon, label: &'static str, active: bool) -> IconButton {
    let state = if active { State::Pressed } else { State::Rest };
    IconButton::new(icon)
        .label(label)
        .help(label)
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::ExtraLarge)
        .state(state)
}

fn indicator(k: &Kit) -> El {
    el().key("nav-indicator")
        .w(INDICATOR_WIDTH)
        .h(INDICATOR_HEIGHT)
        .radius(INDICATOR_WIDTH / 2.0)
        .fill(k.c.bg_accent)
        .a11y_hidden()
        .absolute(0.0, (CELL - INDICATOR_HEIGHT) / 2.0)
}
