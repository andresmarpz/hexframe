mod nav;
mod recordings;
mod settings;
mod status;

use rune::{Cx, El, TextStyle, col, el, row, text};
use rune_kit::Kit;

use crate::ui::{Screen, Ui};

const PAGE_MAX_WIDTH: f32 = 880.0;
const PAGE_GUTTER: f32 = 24.0;
const CARD_RADIUS: f32 = 10.0;

pub(crate) fn root(s: &Ui, cx: &mut Cx<Ui>) -> El {
    let k = Kit::neutral(cx.system_dark());
    let mut layer: Vec<El> = Vec::new();
    let page = match s.screen {
        Screen::Status => status::page(s, &k),
        Screen::Recordings => recordings::page(s, &k),
        Screen::Settings => settings::page(s, &k, &mut layer),
    };
    let body = col()
        .key("window")
        .w_full()
        .h_full()
        .fill(k.c.page)
        .child(nav::bar(s, &k))
        .child(
            col()
                .key("content")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .items_center()
                .child(
                    col()
                        .key("page")
                        .w_full()
                        .max_w(PAGE_MAX_WIDTH)
                        .h_full()
                        .min_h(0.0)
                        .px(PAGE_GUTTER)
                        .pt(20.0)
                        .pb(PAGE_GUTTER)
                        .gap(16.0)
                        .child(page),
                ),
        );
    rune_kit::root(body, layer)
}

fn page_header(k: &Kit, title: &str, subtitle: &str, action: Option<El>) -> El {
    let mut header = row()
        .key("page-header")
        .w_full()
        .center_y()
        .justify_between()
        .child(
            col()
                .gap(2.0)
                .child(text(title).font(title_style(k)).color(k.c.text_base))
                .child(text(subtitle).font(k.ty.body).color(k.c.text_muted)),
        );
    if let Some(action) = action {
        header = header.child(action);
    }
    header
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
