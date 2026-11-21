use std::borrow::Cow;

use greyscreen_core::highlight::{describe_label, dominant_label};
use greyscreen_core::library::EventEntry;
use rune::{Color, El, GrowthSpec, col, el, row, text};
use rune_kit::Kit;

use super::{LIST_PAD, ROW_HEIGHT, faded_scroll, mark_active, muted_note};
use crate::ui::library::format::humanize;
use crate::ui::{EventFilter, ReviewState, Source, Ui, format_clock, seek_target};
use crate::view::review::card::hoverable;
use crate::view::review::tabular;
use crate::view::review::timeline::event_color;

const CHIP_HEIGHT: f32 = 24.0;

pub(crate) fn event_title(event: &EventEntry) -> String {
    match dominant_label(event.highlights.iter().map(String::as_str)) {
        Some(label) => describe_label(label).title.to_owned(),
        None => humanize(&kind_name(event)),
    }
}

fn kind_name(event: &EventEntry) -> String {
    let debug = format!("{:?}", event.kind);
    debug.chars().take_while(|c| c.is_alphanumeric()).collect()
}

fn chip(filter: EventFilter, active: bool, k: &Kit) -> El {
    let chip = row()
        .key(format!("filter-{}", filter.name().to_lowercase()))
        .role("button")
        .label(filter.name())
        .h(CHIP_HEIGHT)
        .px(10.0)
        .center()
        .radius(CHIP_HEIGHT / 2.0)
        .fill(if active {
            k.c.layer_03
        } else {
            Color::BLACK.alpha(0.0)
        })
        .on_press(move |ui: &mut Ui| ui.review_filter(filter))
        .child(
            text(filter.name())
                .font(if active {
                    k.ty.small_medium
                } else {
                    k.ty.small
                })
                .color(if active {
                    k.c.text_base
                } else {
                    k.c.text_muted
                }),
        );
    k.interactive(chip)
}

fn chips(review: &ReviewState, k: &Kit) -> El {
    row()
        .key("event-filters")
        .w_full()
        .px(8.0)
        .py(8.0)
        .gap(4.0)
        .children(
            EventFilter::EVERY
                .into_iter()
                .map(|filter| chip(filter, review.filter == filter, k)),
        )
}

fn event_row(event: &EventEntry, active: bool, k: &Kit) -> El {
    let rec_secs = event.rec_secs;
    let dot = el()
        .size(8.0, 8.0)
        .radius(4.0)
        .fill(event_color(k, &event.highlights));
    let key = format!("event-{}", event.id);
    let line = row()
        .key(key.clone())
        .w_full()
        .h(ROW_HEIGHT)
        .px(12.0)
        .gap(10.0)
        .center_y()
        .on_press(move |ui: &mut Ui| ui.review_seek(seek_target(rec_secs)))
        .child(dot)
        .child(
            text(format_clock(event.rec_secs as u32))
                .font(tabular(k.ty.small))
                .color(k.c.text_muted),
        )
        .child(
            el().grow(1.0).min_w(0.0).child(
                text(event_title(event))
                    .font(if active { k.ty.body_medium } else { k.ty.body })
                    .color(k.c.text_base)
                    .truncate(),
            ),
        );
    k.interactive(hoverable(
        mark_active(
            line,
            active.then(|| format!("event-active-{}", event.id)),
            k,
        ),
        event.id,
        key,
    ))
}

fn events_list(review: &ReviewState, k: &Kit) -> El {
    if review.shown_events().next().is_none() {
        return col()
            .key("events-scroll")
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(muted_note("No events match", k));
    }
    let kit = k.clone();
    let rows = col()
        .w_full()
        .pb(LIST_PAD)
        .growth(GrowthSpec::default())
        .virtual_rows(
            review.shown_events().count(),
            ROW_HEIGHT,
            |s: &Ui, i| {
                let id = s
                    .review
                    .as_ref()
                    .and_then(|r| r.shown_events().nth(i))
                    .map_or(0, |e| e.id);
                Cow::Owned(format!("event-{id}"))
            },
            move |s: &Ui, i| {
                let Some(review) = &s.review else {
                    return el();
                };
                match review.shown_events().nth(i) {
                    Some(event) => {
                        let active = review.active_event() == Some(event.id);
                        event_row(event, active, &kit)
                    }
                    None => el(),
                }
            },
        );
    faded_scroll("events-scroll", rows, false)
}

pub(crate) fn events_body(review: &ReviewState, k: &Kit) -> El {
    match review.source {
        Source::Game => col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(chips(review, k))
            .child(events_list(review, k)),
        Source::Clip(_) => col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(muted_note("Events show while the full game plays", k)),
    }
}
