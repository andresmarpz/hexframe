mod clips;
mod events;
pub(super) mod filter;

use rune::{Color, El, ScrollSpec, col, el, row, text};
use rune_kit::Kit;

use crate::ui::{PanelTab, ReviewState, Ui};
use crate::view::{card, hairline};

#[cfg(test)]
pub(super) use events::event_title;
pub(super) use events::events_body;

pub(super) const PANEL_WIDTH: f32 = 280.0;
const STACKED_HEIGHT: f32 = 360.0;
const ROW_HEIGHT: f32 = 34.0;
const TAB_HEIGHT: f32 = 40.0;
const LIST_PAD: f32 = 8.0;
const FADE: f32 = 14.0;
const ACTIVE_ALPHA: f32 = 0.12;
const ACCENT_BAR: f32 = 2.0;

fn tab_label(review: &ReviewState, tab: PanelTab) -> String {
    match tab {
        PanelTab::Events => {
            let total = review.entry.meta.events.len();
            if review.hidden_events() == 0 {
                format!("Events {total}")
            } else {
                format!("Events {} of {total}", total - review.hidden_events())
            }
        }
        PanelTab::Clips => format!("Clips {}", review.entry.meta.clips.len()),
    }
}

fn tab(review: &ReviewState, which: PanelTab, key: &str, k: &Kit) -> El {
    let active = review.tab == which;
    let underline = if active {
        k.c.bg_accent
    } else {
        Color::BLACK.alpha(0.0)
    };
    let button = col()
        .key(key.to_owned())
        .role("button")
        .label(tab_label(review, which))
        .grow(1.0)
        .h(TAB_HEIGHT)
        .on_press(move |ui: &mut Ui| ui.review_tab(which))
        .child(
            row().w_full().grow(1.0).center().child(
                text(tab_label(review, which))
                    .font(if active { k.ty.body_medium } else { k.ty.body })
                    .color(if active {
                        k.c.text_base
                    } else {
                        k.c.text_muted
                    }),
            ),
        )
        .child(el().w_full().h(2.0).fill(underline).a11y_hidden());
    k.interactive(button)
}

fn tabs(review: &ReviewState, k: &Kit) -> El {
    row()
        .key("panel-tabs")
        .w_full()
        .child(tab(review, PanelTab::Events, "tab-events", k))
        .child(tab(review, PanelTab::Clips, "tab-clips", k))
}

fn accent_bar(key: String, k: &Kit) -> El {
    el().key(key)
        .absolute(0.0, 0.0)
        .size(ACCENT_BAR, ROW_HEIGHT)
        .fill(k.c.bg_accent)
        .a11y_hidden()
}

pub(super) fn mark_active(line: El, active: Option<String>, k: &Kit) -> El {
    if let Some(key) = active {
        line.fill(k.c.bg_accent.alpha(ACTIVE_ALPHA))
            .child(accent_bar(key, k))
    } else {
        line
    }
}

pub(super) fn faded_scroll(key: &str, body: El, pad_bottom: bool) -> El {
    let scroll = col()
        .key(key.to_owned())
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .clip_fade_linear(0.0, FADE, 0.0, FADE)
        .child(el().h(LIST_PAD))
        .child(body);
    if pad_bottom {
        scroll.child(el().h(LIST_PAD))
    } else {
        scroll
    }
}

pub(super) fn muted_note(message: &str, k: &Kit) -> El {
    col().w_full().h(48.0).px(12.0).center_y().child(
        text(message.to_owned())
            .font(k.ty.body)
            .color(k.c.text_muted),
    )
}

pub(super) fn panel(review: &ReviewState, k: &Kit, stacked: bool) -> El {
    let body = match review.tab {
        PanelTab::Events => events_body(review, k),
        PanelTab::Clips => clips::clips_list(review, k),
    };
    let sized = if stacked {
        card(k).key("review-panel").w_full().h(STACKED_HEIGHT)
    } else {
        col()
            .key("review-panel")
            .w(PANEL_WIDTH)
            .h_full()
            .min_h(0.0)
            .fill(k.c.bg_base)
    };
    sized.child(tabs(review, k)).child(hairline(k)).child(body)
}
