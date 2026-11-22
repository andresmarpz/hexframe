use std::borrow::Cow;

use greyscreen_core::highlight::{Highlight, Tone, describe_label, dominant_label};
use greyscreen_engine::FeedItem;
use rune::{El, GrowthSpec, ScrollSpec, col, el, row, text};
use rune_kit::Kit;

use crate::ui::{Ui, format_clock};
use crate::view::widgets::tone_pill;
use crate::view::{card, hairline};

const ROW_HEIGHT: f32 = 36.0;
const BADGE_COLUMN: f32 = 112.0;
const LIST_PADDING: f32 = 8.0;
const TIME_COLUMN: f32 = 48.0;
const NO_HIGHLIGHT_TITLE: &str = "Game";

pub(super) fn badge_title(item: &FeedItem) -> (String, Tone) {
    let labels = item.highlights.iter().map(Highlight::label);
    match dominant_label(labels) {
        Some(label) => {
            let shown = describe_label(label);
            (shown.title.to_owned(), shown.tone)
        }
        None => (NO_HIGHLIGHT_TITLE.to_owned(), Tone::Neutral),
    }
}

pub(super) fn panel(s: &Ui, k: &Kit) -> El {
    let count = s.feed.len();
    let header = row()
        .w_full()
        .h(44.0)
        .px(16.0)
        .center_y()
        .justify_between()
        .child(
            text("Live feed")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            text(format!("{count} events"))
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    let list = if s.feed.is_empty() {
        col().w_full().grow(1.0).center().child(
            text("Game events show up here while you play.")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
    } else {
        scrolling_list(count, k)
    };
    card(k)
        .key("feed-card")
        .grow(1.0)
        .min_h(0.0)
        .child(header)
        .child(hairline(k))
        .child(list)
}

fn scrolling_list(count: usize, k: &Kit) -> El {
    let kit = k.clone();
    let viewport = col()
        .key("feed-scroll")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: true })
        .scroll_edge_fade(16.0)
        .child(
            col()
                .w_full()
                .growth(GrowthSpec::default())
                .py(LIST_PADDING)
                .virtual_rows(
                    count,
                    ROW_HEIGHT,
                    |s: &Ui, i| Cow::Owned(format!("feed-{}", s.feed[i].id)),
                    move |s: &Ui, i| feed_row(&s.feed[i], &kit),
                ),
        );
    k.scroll_bar(viewport)
}

fn feed_row(item: &FeedItem, k: &Kit) -> El {
    let (title, tone) = badge_title(item);
    row()
        .key(format!("feed-{}", item.id))
        .w_full()
        .h(ROW_HEIGHT)
        .px(16.0)
        .gap(12.0)
        .center_y()
        .child(
            el().w(TIME_COLUMN).child(
                text(format_clock(item.game_secs as u32))
                    .font(k.ty.mono)
                    .color(k.c.text_muted),
            ),
        )
        .child(
            row()
                .w(BADGE_COLUMN)
                .center_y()
                .child(tone_pill(k, title, tone)),
        )
        .child(
            text(item.text.clone())
                .font(k.ty.body)
                .color(k.c.text_base)
                .truncate(),
        )
}
