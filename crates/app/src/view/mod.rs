pub(crate) mod champion;
mod delete;
mod games;
mod home;
mod review;
mod row_menu;
mod settings;
mod shell;
pub(crate) mod thumb;

use greyscreen_engine::NoticeLevel;
use rune::{Cx, El, ScrollSpec, TextStyle, col, el, row, text};
use rune_kit::{Kit, Page, SplitView, Toast, ToastRegion, Toolbar};

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
    if let Some((notice, _)) = &s.notice {
        layer.push(toast(notice, &k));
    }
    rune_kit::root(body, layer)
}

fn window(s: &Ui, k: &Kit, cx: &Cx<Ui>) -> El {
    let screen = match s.screen {
        Screen::Home => home_screen(s, k),
        Screen::Games => games_screen(s, k, cx),
    };
    row()
        .key("window")
        .w_full()
        .h_full()
        .fill(k.c.bg_deep)
        .child(shell::sidebar::panel(s, k, cx.titlebar_inset()))
        .child(col().grow(1.0).min_w(0.0).h_full().child(screen))
}

fn home_screen(s: &Ui, k: &Kit) -> El {
    let (main, toolbar) = match s.detail() {
        Detail::Live => (
            fixed_content(home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (scrolling_content(home::page(s, k)), Toolbar::new()),
    };
    Page::new(main).toolbar(toolbar).build(k)
}

fn games_screen(s: &Ui, k: &Kit, cx: &Cx<Ui>) -> El {
    let (main, toolbar) = match (s.detail(), &s.review) {
        (Detail::Game, Some(review)) => (review::page(s, k), review::toolbar(review, k)),
        (Detail::Live, _) => (
            fixed_content(home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (fixed_content(games::pick(s, k)), Toolbar::new()),
    };
    SplitView::new(main)
        .sidebar(
            &s.sidebar,
            |ui: &mut Ui| &mut ui.sidebar,
            games::content(s, k),
        )
        .toolbar(toolbar)
        .reduced_motion(cx.is_reduced_motion())
        .build(k)
}

fn content_column() -> El {
    col()
        .key("content")
        .h_full()
        .grow(1.0)
        .min_w(0.0)
        .items_center()
}

fn page_column() -> El {
    col()
        .key("page")
        .w_full()
        .max_w(PAGE_MAX_WIDTH)
        .px(PAGE_GUTTER)
        .pt(20.0)
        .pb(PAGE_GUTTER)
        .gap(16.0)
}

fn fixed_content(page: El) -> El {
    content_column().child(page_column().h_full().min_h(0.0).child(page))
}

fn scrolling_content(page: El) -> El {
    content_column()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(SCROLL_FADE)
        .child(page_column().child(page))
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
