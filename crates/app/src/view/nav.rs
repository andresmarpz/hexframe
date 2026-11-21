use greyscreen_engine::EngineStatus;
use rune::tok::type_;
use rune::{El, Icon, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::hairline;
use crate::ui::{Screen, Ui, format_clock};

static LOGO: Icon = Icon {
    name: "greyscreen-logo",
    view: 1024.0,
    body: r#"<path fill="currentColor" fill-rule="evenodd" d="M468,101.4A88,88 0 0 1 556,101.4L845.59,268.6A88,88 0 0 1 889.59,344.81L889.59,679.19A88,88 0 0 1 845.59,755.4L556,922.6A88,88 0 0 1 468,922.6L178.41,755.4A88,88 0 0 1 134.41,679.19L134.41,344.81A88,88 0 0 1 178.41,268.6ZM248.6,346.07A24,24 0 0 0 236.6,366.86L236.6,657.14A24,24 0 0 0 248.6,677.93L500,823.07A24,24 0 0 0 524,823.07L775.4,677.93A24,24 0 0 0 787.4,657.14L787.4,366.86A24,24 0 0 0 775.4,346.07L524,200.93A24,24 0 0 0 500,200.93Z"/><path fill="currentColor" d="M682,487.75A28,28 0 0 1 682,536.25L448,671.35A28,28 0 0 1 406,647.1L406,376.9A28,28 0 0 1 448,352.65Z"/>"#,
};

const ITEMS: [(Screen, &str, &str); 3] = [
    (Screen::Status, "nav-status", "Status"),
    (Screen::Recordings, "nav-recordings", "Recordings"),
    (Screen::Settings, "nav-settings", "Settings"),
];

const PILL_HEIGHT: f32 = 26.0;
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
        if screen == Screen::Status && recording_elsewhere(s) {
            tabs = tabs.child(recording_pill(s, k));
        }
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

fn recording_elsewhere(s: &Ui) -> bool {
    s.screen != Screen::Status && matches!(s.status, EngineStatus::Recording { .. })
}

fn recording_pill(s: &Ui, k: &Kit) -> El {
    let clock = format_clock(s.elapsed_secs);
    let font = rune::TextStyle {
        features: "'tnum' 1",
        ..k.ty.ui_medium
    };
    row()
        .key("nav-recording")
        .role("button")
        .label(format!("Recording, {clock}. Go to Status"))
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
        .on_press(|ui: &mut Ui| ui.go(Screen::Status))
}
