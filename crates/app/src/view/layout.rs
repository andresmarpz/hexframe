use rune::{El, ScrollSpec, TextStyle, col, el};
use rune_kit::Kit;

use super::shell;

pub(crate) const TABULAR: &str = "'tnum' 1";
const PAGE_MAX_WIDTH: f32 = 880.0;
const PAGE_GUTTER: f32 = 24.0;
const CARD_RADIUS: f32 = 10.0;
const SCROLL_FADE: f32 = 28.0;

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

pub(super) fn wide_scrolling_content(page: El, top: f32) -> El {
    content_column()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(SCROLL_FADE)
        .child(
            col()
                .key("page")
                .w_full()
                .max_w(PAGE_MAX_WIDTH + 2.0 * PAGE_GUTTER)
                .px(PAGE_GUTTER)
                .pt(top)
                .pb(shell::CLEARANCE)
                .child(page),
        )
}

pub(super) fn card(k: &Kit) -> El {
    col()
        .w_full()
        .fill(k.c.bg_base)
        .radius(CARD_RADIUS)
        .border(1.0, k.c.border_base)
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
