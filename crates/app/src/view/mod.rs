pub(crate) mod champion;
mod delete;
mod home;
mod review;
mod settings;
mod sidebar;

use greyscreen_engine::NoticeLevel;
use rune::{Cx, El, ScrollSpec, TextStyle, col, el};
use rune_kit::{Kit, SplitView, Toast, ToastRegion, Toolbar};

use crate::ui::{Detail, Ui};

const PAGE_MAX_WIDTH: f32 = 880.0;
const PAGE_GUTTER: f32 = 24.0;
const CARD_RADIUS: f32 = 10.0;
const SCROLL_FADE: f32 = 28.0;

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
        settings::sheet(s, &k, &mut layer);
    }
    if let Some(rec) = s.pending_delete_entry() {
        delete::confirm_delete(rec, &k, &mut layer);
    }
    if let Some((notice, _)) = &s.notice {
        layer.push(toast(notice, &k));
    }
    rune_kit::root(body, layer)
}

fn window(s: &Ui, k: &Kit, cx: &Cx<Ui>) -> El {
    let (main, toolbar) = match (s.detail(), &s.review) {
        (Detail::Game, Some(review)) => (review::page(s, k), review::toolbar(review, k)),
        (Detail::Live, _) => (
            fixed_content(home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (scrolling_content(home::page(s, k)), Toolbar::new()),
    };
    SplitView::new(main)
        .sidebar(
            &s.sidebar,
            |ui: &mut Ui| &mut ui.sidebar,
            sidebar::content(s, k),
        )
        .toolbar(toolbar)
        .titlebar_inset(cx.titlebar_inset())
        .reduced_motion(cx.is_reduced_motion())
        .build(k)
        .key("window")
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
