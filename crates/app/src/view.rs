mod clips;
mod games;
mod home;
mod layout;
mod overlays;
mod review;
mod screens;
mod settings;
mod shell;
mod widgets;

pub(crate) use widgets::{champion, thumb};

use greyscreen_core::settings::NavStyle;
use rune::{Cx, El, col, row};
use rune_kit::Kit;

use crate::ui::Ui;
use layout::{TABULAR, card, hairline, title_style, titled_card};
use screens::screen;

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
    review::controls_layer(s, &k, &mut layer);
    if theater.is_none() {
        settings::sheet(
            s,
            &k,
            &mut layer,
            cx.layout_with("window", |r| (r[2], r[3])),
        );
    }
    if theater.is_none() {
        overlays::feedback_dialog(s, &k, &mut layer, cx.layout_with("window", |r| r[3]));
        overlays::export_dialog(s, &k, &mut layer);
    }
    overlays::row_menu(s, &k, &mut layer);
    if let Some(rec) = s.pending_delete_entry() {
        overlays::confirm_delete(rec, &k, &mut layer);
    }
    if let Some(banner) = overlays::update_banner(s, &k) {
        layer.push(banner);
    }
    if let Some((notice, _)) = &s.notice {
        layer.push(overlays::toast(notice, &k));
    }
    rune_kit::root(body, layer)
}

fn window(s: &Ui, k: &Kit, cx: &Cx<Ui>) -> El {
    let review_width = cx.layout_with("review", |r| r[2]);
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
                    review_width,
                )))
        }
        NavStyle::Dock => col()
            .key("window")
            .w_full()
            .h_full()
            .fill(k.c.bg_deep)
            .child(col().w_full().grow(1.0).min_h(0.0).child(screen(
                s,
                k,
                cx.titlebar_inset(),
                review_width,
            )))
            .children(shell::dock::hover_zone(s))
            .child(shell::dock::layer(s, k)),
    }
}
