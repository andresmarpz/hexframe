use greyscreen_engine::EngineStatus;
use rune::tok::type_;
use rune::{El, Icon, TextStyle, col, el, row, text};
use rune_kit::{IconButton, IconButtonVariant, Kit, Sidebar, SidebarRow};

use crate::ui::{Screen, Ui, format_clock};

static LOGO: Icon = Icon {
    name: "greyscreen-logo",
    view: 1024.0,
    body: r#"<path fill="currentColor" fill-rule="evenodd" d="M468,101.4A88,88 0 0 1 556,101.4L845.59,268.6A88,88 0 0 1 889.59,344.81L889.59,679.19A88,88 0 0 1 845.59,755.4L556,922.6A88,88 0 0 1 468,922.6L178.41,755.4A88,88 0 0 1 134.41,679.19L134.41,344.81A88,88 0 0 1 178.41,268.6ZM248.6,346.07A24,24 0 0 0 236.6,366.86L236.6,657.14A24,24 0 0 0 248.6,677.93L500,823.07A24,24 0 0 0 524,823.07L775.4,677.93A24,24 0 0 0 787.4,657.14L787.4,366.86A24,24 0 0 0 775.4,346.07L524,200.93A24,24 0 0 0 500,200.93Z"/><path fill="currentColor" d="M682,487.75A28,28 0 0 1 682,536.25L448,671.35A28,28 0 0 1 406,647.1L406,376.9A28,28 0 0 1 448,352.65Z"/>"#,
};

static HOME: Icon = Icon {
    name: "greyscreen-home",
    view: 16.0,
    body: r#"<path d="M2.5 7.4 8 2.6l5.5 4.8M4 6.4V13h8V6.4M6.5 13V9.5h3V13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

static RECORDINGS: Icon = Icon {
    name: "greyscreen-recordings",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.75" y="3" width="12.5" height="10" rx="2.25" fill="none" stroke="currentColor" stroke-width="1.5"/>"#,
        r#"<path d="M6.8 5.9v4.2l3.4-2.1z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>"#
    ),
};

static GEAR: Icon = Icon {
    name: "greyscreen-gear",
    view: 16.0,
    body: concat!(
        r#"<path d="M6.60 2.78 L6.78 1.11 L9.22 1.11 L9.40 2.78 L10.70 3.32 L12.02 2.27 L13.73 3.98 L12.68 5.30 L13.22 6.60 L14.89 6.78 L14.89 9.22 L13.22 9.40 L12.68 10.70 L13.73 12.02 L12.02 13.73 L10.70 12.68 L9.40 13.22 L9.22 14.89 L6.78 14.89 L6.60 13.22 L5.30 12.68 L3.98 13.73 L2.27 12.02 L3.32 10.70 L2.78 9.40 L1.11 9.22 L1.11 6.78 L2.78 6.60 L3.32 5.30 L2.27 3.98 L3.98 2.27 L5.30 3.32Z" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round"/>"#,
        r#"<circle cx="8" cy="8" r="2.1" fill="none" stroke="currentColor" stroke-width="1.25"/>"#
    ),
};

pub(super) const WIDTH: f32 = 220.0;
const BRAND_HEIGHT: f32 = 52.0;
const LEADING_PAD: f32 = 20.0;
const TRAFFIC_LIGHT_GAP: f32 = 12.0;
const PILL_HEIGHT: f32 = 26.0;

pub(super) fn panel(s: &Ui, k: &Kit, titlebar_inset: f32) -> El {
    col()
        .key("nav")
        .w(WIDTH)
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_deep)
        .child(brand(k, titlebar_inset))
        .child(
            col().w_full().grow(1.0).min_h(0.0).child(
                Sidebar::new()
                    .label("Navigation")
                    .child(item(s, k, Screen::Home, "Home", &HOME, "nav-home"))
                    .child(item(
                        s,
                        k,
                        Screen::Recordings,
                        "Recordings",
                        &RECORDINGS,
                        "nav-recordings",
                    ))
                    .footer(footer(s, k))
                    .build(k),
            ),
        )
}

fn brand(k: &Kit, titlebar_inset: f32) -> El {
    let leading = if titlebar_inset > 0.0 {
        titlebar_inset + TRAFFIC_LIGHT_GAP
    } else {
        LEADING_PAD
    };
    row()
        .key("nav-top")
        .w_full()
        .h(BRAND_HEIGHT)
        .pl(leading)
        .pr(LEADING_PAD)
        .center_y()
        .window_drag()
        .child(
            row()
                .key("nav-brand")
                .gap(10.0)
                .center_y()
                .child(rune::icon(&LOGO, 20.0).color(k.c.bg_accent).a11y_hidden())
                .child(text("Greyscreen").font(type_::SECTION).color(k.c.text_base)),
        )
}

fn item(
    s: &Ui,
    k: &Kit,
    screen: Screen,
    label: &'static str,
    icon: &'static Icon,
    key: &'static str,
) -> El {
    let active = s.screen == screen || (screen == Screen::Recordings && s.screen == Screen::Review);
    SidebarRow::new(label)
        .icon(icon)
        .selected(active)
        .on_select(move |ui: &mut Ui, _| ui.go(screen))
        .build(k)
        .key(key)
}

fn footer(s: &Ui, k: &Kit) -> El {
    let on_settings = s.screen == Screen::Settings;
    let variant = if on_settings {
        IconButtonVariant::Neutral
    } else {
        IconButtonVariant::GhostMuted
    };
    let settings = IconButton::new(&GEAR)
        .label("Settings")
        .help("Settings")
        .variant(variant)
        .on_press(|ui: &mut Ui| ui.go(Screen::Settings))
        .build(k)
        .key("nav-settings");
    let mut bar = row()
        .key("nav-footer")
        .w_full()
        .h(32.0)
        .px(8.0)
        .center_y()
        .justify_between();
    bar = bar.child(if recording_elsewhere(s) {
        recording_pill(s, k)
    } else {
        el().a11y_hidden()
    });
    bar.child(settings)
}

fn recording_elsewhere(s: &Ui) -> bool {
    s.screen != Screen::Home && matches!(s.status, EngineStatus::Recording { .. })
}

fn recording_pill(s: &Ui, k: &Kit) -> El {
    let clock = format_clock(s.elapsed_secs);
    let font = TextStyle {
        features: "'tnum' 1",
        ..k.ty.ui_medium
    };
    row()
        .key("nav-recording")
        .role("button")
        .label(format!("Recording, {clock}. Go to Home"))
        .focusable(true)
        .h(PILL_HEIGHT)
        .px(10.0)
        .gap(6.0)
        .center_y()
        .radius(PILL_HEIGHT / 2.0)
        .fill(k.c.danger.bg)
        .child(
            el().size(8.0, 8.0)
                .radius(4.0)
                .fill(k.c.danger.icon)
                .a11y_hidden(),
        )
        .child(text(clock).font(font).color(k.c.danger.fg).a11y_hidden())
        .on_press(|ui: &mut Ui| ui.go(Screen::Home))
}
