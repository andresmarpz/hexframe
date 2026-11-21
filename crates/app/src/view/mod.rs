pub(crate) mod champion;
mod clips;
mod delete;
mod games;
mod grip;
mod home;
mod review;
mod row_menu;
mod settings;
mod shell;
pub(crate) mod thumb;
mod update_banner;

use greyscreen_core::settings::NavStyle;
use greyscreen_engine::NoticeLevel;
use rune::{Cx, El, ScrollSpec, TextStyle, col, el, row, text};
use rune_kit::{Divider, Kit, Page, Toast, ToastRegion, Toolbar};

use crate::ui::{Detail, Screen, Ui};

const PAGE_MAX_WIDTH: f32 = 880.0;
const PAGE_GUTTER: f32 = 24.0;
const CARD_RADIUS: f32 = 10.0;
const SCROLL_FADE: f32 = 28.0;
const SECTION_HEADER_HEIGHT: f32 = 44.0;

pub(crate) fn root(s: &Ui, cx: &mut Cx<Ui>) -> El {
    let k = Kit::neutral(cx.system_dark());
    let mut layer: Vec<El> = Vec::new();
    let theater = s.review.as_ref().filter(|r| r.theater);
    let body = match theater {
        Some(review) => review::theater(review),
        None => window(s, &k, cx),
    };
    if let Some(card) = review::hover_card(s, &k) {
        layer.push(card);
    }
    if theater.is_none() {
        settings::sheet(s, &k, &mut layer, cx.layout_with("window", |r| r[3]));
    }
    row_menu::menu(s, &k, &mut layer);
    if let Some(rec) = s.pending_delete_entry() {
        delete::confirm_delete(rec, &k, &mut layer);
    }
    if let Some(banner) = update_banner::layer(s, &k) {
        layer.push(banner);
    }
    if let Some((notice, _)) = &s.notice {
        layer.push(toast(notice, &k));
    }
    rune_kit::root(body, layer)
}

fn window(s: &Ui, k: &Kit, cx: &Cx<Ui>) -> El {
    match s.settings.nav_style {
        NavStyle::Sidebar => {
            let folded = shell::sidebar::is_folded(s, cx.layout_with("window", |r| r[2]));
            row()
                .key("window")
                .w_full()
                .h_full()
                .fill(k.c.bg_deep)
                .child(shell::sidebar::panel(s, k, folded, cx.titlebar_inset()))
                .child(col().grow(1.0).min_w(0.0).h_full().child(screen(
                    s,
                    k,
                    shell::sidebar::lights_overhang(folded, cx.titlebar_inset()),
                )))
        }
        NavStyle::Dock => col()
            .key("window")
            .w_full()
            .h_full()
            .fill(k.c.bg_deep)
            .child(
                col()
                    .w_full()
                    .grow(1.0)
                    .min_h(0.0)
                    .child(screen(s, k, cx.titlebar_inset())),
            )
            .children(shell::dock::hover_zone(s))
            .child(shell::dock::layer(s, k)),
    }
}

fn screen(s: &Ui, k: &Kit, inset: f32) -> El {
    match s.screen {
        Screen::Home => home_screen(s, k, inset),
        Screen::Games => games_screen(s, k, inset),
        Screen::Clips => clips_screen(s, k, inset),
    }
}

fn home_screen(s: &Ui, k: &Kit, inset: f32) -> El {
    let (main, toolbar) = match s.detail() {
        Detail::Live => (
            fixed_content(s, home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (scrolling_content(s, home::page(s, k)), Toolbar::new()),
    };
    Page::new(main)
        .toolbar(toolbar)
        .titlebar_inset(inset)
        .build(k)
}

fn clips_screen(s: &Ui, k: &Kit, inset: f32) -> El {
    Page::new(scrolling_content(s, clips::page(s, k)))
        .title("Clips")
        .titlebar_inset(inset)
        .build(k)
}

fn games_screen(s: &Ui, k: &Kit, inset: f32) -> El {
    let (main, toolbar) = match (s.detail(), &s.review) {
        (Detail::Game, Some(review)) => (review_page(s, k), review::toolbar(review, k)),
        (Detail::Live, _) => (
            fixed_content(s, home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (fixed_content(s, games::pick(s, k)), Toolbar::new()),
    };
    let body = row()
        .key("games-body")
        .w_full()
        .h_full()
        .child(games::list(s, k))
        .child(Divider::vertical().build(k))
        .child(col().grow(1.0).min_w(0.0).h_full().child(main));
    Page::new(body)
        .toolbar(toolbar)
        .titlebar_inset(inset)
        .build(k)
}

fn review_page(s: &Ui, k: &Kit) -> El {
    let page = review::page(s, k);
    if s.settings.nav_style == NavStyle::Sidebar {
        return page;
    }
    let room = if s.dock.visible {
        shell::dock::REVIEW_ROOM
    } else {
        0.0
    };
    col()
        .w_full()
        .h_full()
        .child(page.grow(1.0).min_h(0.0))
        .child(el().key("dock-reserve").w_full().h(room).a11y_hidden())
}

fn content_column() -> El {
    col()
        .key("content")
        .h_full()
        .grow(1.0)
        .min_w(0.0)
        .items_center()
}

fn page_column(s: &Ui) -> El {
    col()
        .key("page")
        .w_full()
        .max_w(PAGE_MAX_WIDTH)
        .px(PAGE_GUTTER)
        .pt(20.0)
        .pb(PAGE_GUTTER.max(dock_clearance(s)))
        .gap(16.0)
}

fn dock_clearance(s: &Ui) -> f32 {
    match s.settings.nav_style {
        NavStyle::Sidebar => 0.0,
        NavStyle::Dock => shell::dock::CLEARANCE,
    }
}

fn fixed_content(s: &Ui, page: El) -> El {
    content_column().child(page_column(s).h_full().min_h(0.0).child(page))
}

fn scrolling_content(s: &Ui, page: El) -> El {
    content_column()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(SCROLL_FADE)
        .child(page_column(s).child(page))
}

fn card(k: &Kit) -> El {
    col()
        .w_full()
        .fill(k.c.bg_base)
        .radius(CARD_RADIUS)
        .border(1.0, k.c.border_base)
}

fn section_header(k: &Kit, title: &str) -> El {
    row()
        .w_full()
        .h(SECTION_HEADER_HEIGHT)
        .px(16.0)
        .center_y()
        .child(text(title).font(k.ty.body_medium).color(k.c.text_base))
}

fn hairline(k: &Kit) -> El {
    el().w_full().h(1.0).fill(k.c.border_muted).a11y_hidden()
}

fn title_style(k: &Kit) -> TextStyle {
    TextStyle {
        size: 22.0,
        line_height: 28.0,
        ..k.ty.body_medium
    }
}

fn toast(notice: &crate::ui::Notice, k: &Kit) -> El {
    let built = match notice.level {
        NoticeLevel::Info => Toast::new("notice", notice.text.clone()),
        NoticeLevel::Warn => Toast::new("notice", "Warning").description(notice.text.clone()),
        NoticeLevel::Error => Toast::new("notice", "Error").description(notice.text.clone()),
    };
    let built = built.closable().on_close(|ui: &mut Ui| ui.notice = None);
    ToastRegion::new().toast(built.build(k)).build(k)
}
