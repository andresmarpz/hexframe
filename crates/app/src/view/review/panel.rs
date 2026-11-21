use std::borrow::Cow;

use greyscreen_core::library::{ClipEntry, EventEntry};
use rune::{El, GrowthSpec, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::card::hoverable;
use super::tabular;
use super::timeline::event_color;
use crate::ui::{ReviewState, Source, Ui, format_clock, seek_target};
use crate::view::{card, hairline};

const PANEL_WIDTH: f32 = 280.0;
const ROW_HEIGHT: f32 = 34.0;
const CLIPS_HEIGHT: f32 = 150.0;

pub(super) fn event_label(event: &EventEntry) -> String {
    if !event.highlights.is_empty() {
        return event
            .highlights
            .iter()
            .map(|h| humanize(h))
            .collect::<Vec<_>>()
            .join(", ");
    }
    let debug = format!("{:?}", event.kind);
    let name: String = debug.chars().take_while(|c| c.is_alphanumeric()).collect();
    humanize(&name)
}

fn humanize(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn section_title(k: &Kit, label: &str) -> El {
    row().w_full().h(36.0).px(12.0).center_y().child(
        text(label.to_owned())
            .font(k.ty.small_medium)
            .color(k.c.text_muted),
    )
}

fn event_row(event: &EventEntry, k: &Kit) -> El {
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
                text(event_label(event))
                    .font(k.ty.body)
                    .color(k.c.text_base)
                    .truncate(),
            ),
        );
    k.interactive(hoverable(line, event.id, key))
}

fn events_list(review: &ReviewState, k: &Kit) -> El {
    let kit = k.clone();
    col()
        .key("events-scroll")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(col().w_full().growth(GrowthSpec::default()).virtual_rows(
            review.entry.meta.events.len(),
            ROW_HEIGHT,
            |s: &Ui, i| {
                let id = s
                    .review
                    .as_ref()
                    .and_then(|r| r.entry.meta.events.get(i))
                    .map_or(0, |e| e.id);
                Cow::Owned(format!("event-{id}"))
            },
            move |s: &Ui, i| match s.review.as_ref().and_then(|r| r.entry.meta.events.get(i)) {
                Some(event) => event_row(event, &kit),
                None => el(),
            },
        ))
}

fn clip_row(index: usize, clip: &ClipEntry, active: bool, k: &Kit) -> El {
    let line = row()
        .key(format!("clip-{index}"))
        .w_full()
        .h(ROW_HEIGHT)
        .px(12.0)
        .gap(10.0)
        .center_y()
        .on_press(move |ui: &mut Ui| ui.play_clip(index))
        .child(
            el().grow(1.0).min_w(0.0).child(
                text(super::panel::humanize(&clip.label))
                    .font(if active { k.ty.body_medium } else { k.ty.body })
                    .color(if active {
                        k.c.text_accent
                    } else {
                        k.c.text_base
                    })
                    .truncate(),
            ),
        )
        .child(
            text(format!(
                "{} - {}",
                format_clock(clip.start_secs as u32),
                format_clock(clip.end_secs as u32)
            ))
            .font(tabular(k.ty.small))
            .color(k.c.text_muted),
        );
    k.interactive(line)
}

fn clips_list(review: &ReviewState, k: &Kit) -> El {
    let rows = review
        .entry
        .meta
        .clips
        .iter()
        .enumerate()
        .map(|(i, clip)| clip_row(i, clip, review.source == Source::Clip(i), k));
    let body = if review.entry.meta.clips.is_empty() {
        col().w_full().h(48.0).px(12.0).child(
            text("No clips for this game")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
    } else {
        col().w_full().children(rows)
    };
    col()
        .key("clips-scroll")
        .w_full()
        .h(CLIPS_HEIGHT)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(body)
}

fn clip_notice(review: &ReviewState, k: &Kit) -> El {
    let label = match review.source {
        Source::Clip(i) => review
            .entry
            .meta
            .clips
            .get(i)
            .map(|c| humanize(&c.label))
            .unwrap_or_default(),
        Source::Game => String::new(),
    };
    col()
        .key("clip-notice")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .px(12.0)
        .gap(10.0)
        .child(
            text(format!("Playing clip: {label}"))
                .font(k.ty.body)
                .color(k.c.text_base),
        )
        .child(
            Button::new("Back to full game")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Small)
                .on_press(|ui: &mut Ui| ui.play_game())
                .build(k)
                .key("review-game"),
        )
}

pub(super) fn panel(review: &ReviewState, k: &Kit) -> El {
    let upper = match review.source {
        Source::Game => col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(section_title(
                k,
                &format!("Events ({})", review.entry.meta.events.len()),
            ))
            .child(events_list(review, k)),
        Source::Clip(_) => col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(section_title(k, "Events"))
            .child(clip_notice(review, k)),
    };
    card(k)
        .key("review-panel")
        .w(PANEL_WIDTH)
        .h_full()
        .min_h(0.0)
        .child(upper)
        .child(hairline(k))
        .child(section_title(k, "Clips"))
        .child(clips_list(review, k))
}
