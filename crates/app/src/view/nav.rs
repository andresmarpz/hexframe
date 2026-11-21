use rune::tok::type_;
use rune::{El, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::hairline;
use crate::ui::{Screen, Ui};

const ITEMS: [(Screen, &str, &str); 3] = [
    (Screen::Status, "nav-status", "Status"),
    (Screen::Recordings, "nav-recordings", "Recordings"),
    (Screen::Settings, "nav-settings", "Settings"),
];

pub(super) fn bar(s: &Ui, k: &Kit) -> El {
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
                .px(20.0)
                .center_y()
                .justify_between()
                .child(
                    row()
                        .gap(10.0)
                        .center_y()
                        .child(
                            el().size(10.0, 10.0)
                                .radius(3.0)
                                .fill(k.c.bg_accent)
                                .a11y_hidden(),
                        )
                        .child(text("Greyscreen").font(type_::SECTION).color(k.c.text_base)),
                )
                .child(tabs),
        )
        .child(hairline(k))
}
