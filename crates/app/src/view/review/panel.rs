use std::borrow::Cow;

use greyscreen_core::highlight::{describe_label, dominant_label};
use greyscreen_core::library::{ClipEntry, EventEntry};
use greyscreen_engine::reveal;
use rune::{Color, El, GrowthSpec, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, glyphs};

use super::card::hoverable;
use super::tabular;
use super::timeline::event_color;
use crate::ui::library_format::{clip_title, humanize};
use crate::ui::{EventFilter, PanelTab, ReviewState, Source, Ui, format_clock, seek_target};
use crate::view::{card, hairline, thumb};

pub(super) const PANEL_WIDTH: f32 = 280.0;
const ROW_HEIGHT: f32 = 34.0;
const CLIP_ROW_HEIGHT: f32 = 52.0;
const CLIP_THUMB_WIDTH: f32 = 64.0;
const CLIP_THUMB_HEIGHT: f32 = 36.0;
const TAB_HEIGHT: f32 = 40.0;
const CHIP_HEIGHT: f32 = 24.0;
const LIST_PAD: f32 = 8.0;
const FADE: f32 = 14.0;
const ACTIVE_ALPHA: f32 = 0.12;
const ACCENT_BAR: f32 = 2.0;

pub(super) fn event_title(event: &EventEntry) -> String {
    match dominant_label(event.highlights.iter().map(String::as_str)) {
        Some(label) => describe_label(label).title.to_owned(),
        None => humanize(&kind_name(event)),
    }
}

fn kind_name(event: &EventEntry) -> String {
    let debug = format!("{:?}", event.kind);
    debug.chars().take_while(|c| c.is_alphanumeric()).collect()
}

fn tab_label(review: &ReviewState, tab: PanelTab) -> String {
    match tab {
        PanelTab::Events => {
            let total = review.entry.meta.events.len();
            if review.filter == EventFilter::All {
                format!("Events {total}")
            } else {
                format!("Events {} of {total}", review.shown_events().count())
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

fn accent_bar(key: String, k: &Kit) -> El {
    el().key(key)
        .absolute(0.0, 0.0)
        .size(ACCENT_BAR, ROW_HEIGHT)
        .fill(k.c.bg_accent)
        .a11y_hidden()
}

fn mark_active(line: El, active: Option<String>, k: &Kit) -> El {
    if let Some(key) = active {
        line.fill(k.c.bg_accent.alpha(ACTIVE_ALPHA))
            .child(accent_bar(key, k))
    } else {
        line
    }
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

fn faded_scroll(key: &str, body: El, pad_bottom: bool) -> El {
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

fn muted_note(message: &str, k: &Kit) -> El {
    col().w_full().h(48.0).px(12.0).center_y().child(
        text(message.to_owned())
            .font(k.ty.body)
            .color(k.c.text_muted),
    )
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

pub(super) fn events_body(review: &ReviewState, k: &Kit) -> El {
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

fn clip_row(index: usize, clip: &ClipEntry, active: bool, review: &ReviewState, k: &Kit) -> El {
    let file = review.entry.folder.join(&clip.file);
    let show = IconButton::new(&glyphs::FOLDER)
        .label("Show clip in folder")
        .help("Show in folder")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(move |_: &mut Ui| reveal(&file))
        .build(k)
        .key(format!("clip-folder-{index}"))
        .group_opacity(0.0, 1.0);
    let line = row()
        .key(format!("clip-{index}"))
        .w_full()
        .h(CLIP_ROW_HEIGHT)
        .px(12.0)
        .gap(10.0)
        .center_y()
        .hover_group()
        .on_press(move |ui: &mut Ui| ui.play_clip(index))
        .child(thumb::clip_thumb_small(
            k,
            &review.entry,
            clip,
            (CLIP_THUMB_WIDTH, CLIP_THUMB_HEIGHT),
        ))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(
                    text(clip_title(&clip.label))
                        .font(if active { k.ty.body_medium } else { k.ty.body })
                        .color(if active {
                            k.c.text_accent
                        } else {
                            k.c.text_base
                        })
                        .truncate(),
                )
                .child(
                    text(format!(
                        "{} - {}",
                        format_clock(clip.start_secs as u32),
                        format_clock(clip.end_secs as u32)
                    ))
                    .font(tabular(k.ty.small))
                    .color(k.c.text_muted),
                ),
        )
        .child(show);
    k.interactive(mark_active(
        line,
        active.then(|| format!("clip-active-{index}")),
        k,
    ))
}

fn clips_list(review: &ReviewState, k: &Kit) -> El {
    if review.entry.meta.clips.is_empty() {
        return col()
            .key("clips-scroll")
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(muted_note("No clips for this game", k));
    }
    let active = review.active_clip();
    let rows = col().w_full().children(
        review
            .entry
            .meta
            .clips
            .iter()
            .enumerate()
            .map(|(i, clip)| clip_row(i, clip, active == Some(i), review, k)),
    );
    faded_scroll("clips-scroll", rows, true)
}

fn clip_banner(review: &ReviewState, k: &Kit) -> Option<El> {
    let Source::Clip(i) = review.source else {
        return None;
    };
    let label = review
        .entry
        .meta
        .clips
        .get(i)
        .map(|c| clip_title(&c.label))
        .unwrap_or_default();
    Some(
        row()
            .key("clip-notice")
            .w_full()
            .px(12.0)
            .py(8.0)
            .gap(8.0)
            .center_y()
            .child(
                el().grow(1.0).min_w(0.0).child(
                    text(format!("Playing clip: {label}"))
                        .font(k.ty.small)
                        .color(k.c.text_base)
                        .truncate(),
                ),
            )
            .child(
                Button::new("Full game")
                    .variant(ButtonVariant::Neutral)
                    .control_size(ControlSize::Small)
                    .on_press(|ui: &mut Ui| ui.play_game())
                    .build(k)
                    .key("review-game"),
            ),
    )
}

pub(super) fn panel(review: &ReviewState, k: &Kit) -> El {
    let body = match review.tab {
        PanelTab::Events => events_body(review, k),
        PanelTab::Clips => clips_list(review, k),
    };
    let mut panel = card(k)
        .key("review-panel")
        .w(PANEL_WIDTH)
        .h_full()
        .min_h(0.0)
        .child(tabs(review, k))
        .child(hairline(k));
    if let Some(banner) = clip_banner(review, k) {
        panel = panel.child(banner).child(hairline(k));
    }
    panel.child(body)
}
