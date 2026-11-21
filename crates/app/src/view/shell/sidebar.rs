pub(super) mod icons;
mod recent;
mod status;
mod storage;

use rune::tok::type_;
use rune::{El, Icon, ScrollSpec, col, el, row, text};
use rune_kit::{Kit, SidebarRow};

use self::icons::{CLIPS, GAMES, GEAR, HOME, LOGO};
use crate::ui::{Screen, Ui};

pub(in crate::view) const WIDTH: f32 = 220.0;
const BRAND_HEIGHT: f32 = 48.0;
const LIGHTS_HEIGHT: f32 = 40.0;
const PAD: f32 = 12.0;

pub(in crate::view) fn panel(s: &Ui, k: &Kit, titlebar_inset: f32) -> El {
    let list = col()
        .key("nav-list")
        .role("list")
        .label("Navigation")
        .focus_group()
        .focus_follows()
        .w_full()
        .px(PAD - 4.0)
        .gap(2.0)
        .child(item(
            k,
            "Home",
            &HOME,
            "nav-home",
            s.screen == Screen::Home,
            None,
            |ui| ui.go(Screen::Home),
        ))
        .child(item(
            k,
            "Games",
            &GAMES,
            "nav-games",
            s.screen == Screen::Games,
            count(s.recordings.len()),
            |ui| ui.go(Screen::Games),
        ))
        .child(item(
            k,
            "Clips",
            &CLIPS,
            "nav-clips",
            s.screen == Screen::Clips,
            count(s.clip_count()),
            |ui| ui.go(Screen::Clips),
        ));
    col()
        .key("nav")
        .w(WIDTH)
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_deep)
        .child(brand(k, titlebar_inset))
        .child(col().w_full().px(PAD).pb(10.0).child(status::block(s, k)))
        .child(middle(s, k, list))
        .child(
            col()
                .key("nav-footer")
                .w_full()
                .px(PAD - 4.0)
                .py(PAD - 4.0)
                .gap(8.0)
                .child(storage::block(s, k))
                .child(settings(s, k)),
        )
}

fn middle(s: &Ui, k: &Kit, list: El) -> El {
    let mut body = col().w_full().gap(12.0).child(list);
    if let Some(recent) = recent::section(s, k) {
        body = body.child(col().w_full().px(PAD - 4.0).focus_group().child(recent));
    }
    col()
        .key("nav-middle")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(body)
}

fn count(n: usize) -> Option<usize> {
    (n > 0).then_some(n)
}

fn item(
    k: &Kit,
    label: &'static str,
    icon: &'static Icon,
    key: &'static str,
    selected: bool,
    badge: Option<usize>,
    go: fn(&mut Ui),
) -> El {
    let mut row = SidebarRow::new(label)
        .icon(icon)
        .selected(selected)
        .on_select(move |ui: &mut Ui, _| go(ui));
    if let Some(n) = badge {
        row = row.badge(n.to_string());
    }
    row.build(k).key(key)
}

fn settings(s: &Ui, k: &Kit) -> El {
    SidebarRow::new("Settings")
        .icon(&GEAR)
        .selected(s.settings_open)
        .on_select(|ui: &mut Ui, _| ui.toggle_settings())
        .build(k)
        .key("nav-settings")
}

fn wordmark(k: &Kit) -> El {
    row()
        .key("nav-brand")
        .gap(10.0)
        .center_y()
        .child(rune::icon(&LOGO, 20.0).color(k.c.bg_accent).a11y_hidden())
        .child(text("Greyscreen").font(type_::SECTION).color(k.c.text_base))
}

fn brand(k: &Kit, titlebar_inset: f32) -> El {
    if titlebar_inset > 0.0 {
        return col()
            .w_full()
            .child(el().w_full().h(LIGHTS_HEIGHT).window_drag().a11y_hidden())
            .child(
                row()
                    .w_full()
                    .h(BRAND_HEIGHT - 12.0)
                    .px(PAD + 4.0)
                    .center_y()
                    .window_drag()
                    .child(wordmark(k)),
            );
    }
    row()
        .w_full()
        .h(BRAND_HEIGHT)
        .px(PAD + 4.0)
        .center_y()
        .window_drag()
        .child(wordmark(k))
}
