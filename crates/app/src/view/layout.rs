use rune::{El, ScrollSpec, TextStyle, col, el, row, text};
use rune_kit::Kit;

use super::shell;

pub(crate) const TABULAR: &str = "'tnum' 1";
const PAGE_MAX_WIDTH: f32 = 880.0;
const PAGE_GUTTER: f32 = 24.0;
const CARD_RADIUS: f32 = 10.0;
const SCROLL_FADE: f32 = 28.0;
const SECTION_HEADER_HEIGHT: f32 = 44.0;

pub(super) fn content_column() -> El {
    col()
        .key("content")
        .h_full()
        .grow(1.0)
        .min_w(0.0)
        .items_center()
}

pub(super) fn page_column() -> El {
    col()
        .key("page")
        .w_full()
        .max_w(PAGE_MAX_WIDTH)
        .px(PAGE_GUTTER)
        .pt(20.0)
        .pb(shell::CLEARANCE)
        .gap(16.0)
}

pub(super) fn fixed_content(page: El) -> El {
    content_column().child(page_column().h_full().min_h(0.0).child(page))
}

pub(super) fn scrolling_content(page: El) -> El {
    content_column()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(SCROLL_FADE)
        .child(page_column().child(page))
}

pub(super) fn card(k: &Kit) -> El {
    col()
        .w_full()
        .fill(k.c.bg_base)
        .radius(CARD_RADIUS)
        .border(1.0, k.c.border_base)
}

pub(super) fn section_header(k: &Kit, title: &str) -> El {
    row()
        .w_full()
        .h(SECTION_HEADER_HEIGHT)
        .px(16.0)
        .center_y()
        .child(text(title).font(k.ty.body_medium).color(k.c.text_base))
}

pub(super) fn titled_card(k: &Kit, key: &'static str, title: &str) -> El {
    card(k)
        .key(key)
        .child(section_header(k, title))
        .child(hairline(k))
}

pub(super) fn hairline(k: &Kit) -> El {
    el().w_full().h(1.0).fill(k.c.border_muted).a11y_hidden()
}

pub(super) fn title_style(k: &Kit) -> TextStyle {
    TextStyle {
        size: 22.0,
        line_height: 28.0,
        ..k.ty.body_medium
    }
}
