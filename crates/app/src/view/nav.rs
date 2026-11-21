use rune::tok::type_;
use rune::{El, Icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::hairline;
use crate::ui::{Screen, Ui};

static LOGO: Icon = Icon {
    name: "greyscreen-logo",
    view: 64.0,
    body: r#"<polygon points="32,8 52.8,20 52.8,44 32,56 11.2,44 11.2,20" stroke="currentColor" stroke-width="6" stroke-linejoin="round" fill="none"/><polygon points="27,23 27,41 42,32" fill="currentColor" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>"#,
};

const ITEMS: [(Screen, &str, &str); 3] = [
    (Screen::Status, "nav-status", "Status"),
    (Screen::Recordings, "nav-recordings", "Recordings"),
    (Screen::Settings, "nav-settings", "Settings"),
];

const LEADING_PAD: f32 = 20.0;
const TRAFFIC_LIGHT_GAP: f32 = 12.0;

pub(super) fn bar(s: &Ui, k: &Kit, titlebar_inset: f32) -> El {
    let leading = if titlebar_inset > 0.0 {
        titlebar_inset + TRAFFIC_LIGHT_GAP
    } else {
        LEADING_PAD
    };
    let mut tabs = row().key("nav-tabs").gap(4.0).center_y();
    for (screen, key, label) in ITEMS {
        let active =
            s.screen == screen || (screen == Screen::Recordings && s.screen == Screen::Review);
        let variant = if active {
            ButtonVariant::Neutral
        } else {
            ButtonVariant::Ghost
        };
        tabs = tabs.child(
            Button::new(label)
                .variant(variant)
                .control_size(ControlSize::Large)
                .on_press(move |ui: &mut Ui| ui.go(screen))
                .build(k)
                .key(key),
        );
    }
    rune::col()
        .key("nav")
        .w_full()
        .child(
            row()
                .w_full()
                .h(52.0)
                .pl(leading)
                .pr(LEADING_PAD)
                .center_y()
                .justify_between()
                .child(
                    row()
                        .key("nav-brand")
                        .gap(10.0)
                        .center_y()
                        .child(rune::icon(&LOGO, 20.0).color(k.c.bg_accent).a11y_hidden())
                        .child(text("Greyscreen").font(type_::SECTION).color(k.c.text_base)),
                )
                .child(tabs),
        )
        .child(hairline(k))
}
