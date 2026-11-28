mod clips;
mod game_page;
mod games;
mod home;
mod layout;
mod overlays;
mod review;
mod screens;
mod settings;
pub(crate) mod shell;
mod widgets;

pub(crate) use widgets::{champion, icon_sheet, thumb, timeline_sheet};

use rune::{Cx, El, col};
use rune_kit::Kit;

use crate::ui::Ui;
use layout::{TABULAR, card, hairline, title_style};
use screens::screen;

pub(crate) fn root(s: &Ui, cx: &mut Cx<Ui>) -> El {
    let k = Kit::neutral(cx.system_dark());
    if icon_sheet::shown() {
        return rune_kit::root(icon_sheet::sheet(&k, s.recordings.first()), Vec::new());
    }
    if timeline_sheet::shown() {
        return rune_kit::root(timeline_sheet::sheet(&k), Vec::new());
    }
    let mut layer: Vec<El> = Vec::new();
    let theater = s.review.as_ref().filter(|r| r.theater);
    let body = match theater {
        Some(review) => review::theater(review),
        None => window(s, &k, cx),
    };
    if let Some(card) = review::hover_card(s, &k) {
        layer.push(card);
    }
    if let Some(card) = review::seek_preview(s, &k) {
        layer.push(card);
    }
    review::controls_layer(s, &k, &mut layer);
    if theater.is_none() {
        shell::overlay(s, &k, &mut layer);
    }
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
    rune_kit::root(body, layer).on_layout(|s: &mut Ui, r| s.note_window_size(r[2], r[3]))
}

fn window(s: &Ui, k: &Kit, cx: &Cx<Ui>) -> El {
    let review_width = cx.layout_with("review", |r| r[2]);
    col()
        .key("window")
        .w_full()
        .h_full()
        .fill(k.c.bg_deep)
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(screen(s, k, review_width)),
        )
        .children(shell::hover_zone(s))
        .children(shell::layer(s, k))
        .child(shell::top_bar(s, k, cx))
        .instant_changes()
}
