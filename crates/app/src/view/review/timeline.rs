use greyscreen_core::highlight::{Tone, describe_label, dominant_label};
use greyscreen_core::library::{ClipEntry, EventEntry};
use rune::{Color, Drag, El, Phase, el};
use rune_kit::Kit;

use super::card::hoverable;
use crate::ui::{ReviewState, Source, Ui, seek_target};
use crate::view::widgets::tone_color;

const HEIGHT: f32 = 28.0;
const TRACK_HEIGHT: f32 = 4.0;
const BAND_HEIGHT: f32 = 10.0;
const DOT: f32 = 6.0;
const MARKER_WIDTH: f32 = 14.0;
const MARKER_HEIGHT: f32 = 16.0;
const THUMB: f32 = 12.0;
const THUMB_HOVER: f32 = 14.0;
const BAND_ALPHA: f32 = 0.22;

pub(super) fn event_color(k: &Kit, highlights: &[String]) -> Color {
    let tone = dominant_label(highlights.iter().map(String::as_str))
        .map_or(Tone::Neutral, |label| describe_label(label).tone);
    tone_color(k, tone)
}

pub(super) fn fraction(secs: f64, duration: f64) -> f32 {
    if duration <= 0.0 {
        return 0.0;
    }
    (secs / duration).clamp(0.0, 1.0) as f32
}

pub(super) fn drag_target(x: f32, width: f32, duration: f64) -> f64 {
    if width <= 0.0 {
        return 0.0;
    }
    f64::from((x / width).clamp(0.0, 1.0)) * duration
}

fn rail(item: El, size: f32, height: f32, progress: f32) -> El {
    rune::row()
        .pin(Some(0.0), Some(0.0), Some(0.0), None)
        .h(height)
        .px(size / 2.0)
        .center_y()
        .child(el().w_frac(progress).h(height))
        .child(item)
}

pub(super) fn thumb_rail(thumb: El, size: f32, height: f32, progress: f32) -> El {
    rail(thumb.ml(-size / 2.0), size, height, progress)
        .instant_changes()
        .a11y_hidden()
}

fn marker(event: &EventEntry, review: &ReviewState, k: &Kit) -> El {
    let rec_secs = event.rec_secs;
    let key = format!("marker-{}", event.id);
    let marker = el()
        .key(key.clone())
        .size(MARKER_WIDTH, MARKER_HEIGHT)
        .center()
        .on_press(move |ui: &mut Ui| ui.review_seek(seek_target(rec_secs)))
        .child(
            el().size(DOT, DOT)
                .radius(DOT / 2.0)
                .fill(event_color(k, &event.highlights)),
        );
    rail(
        hoverable(marker.ml(-MARKER_WIDTH / 2.0), event.id, key),
        MARKER_WIDTH,
        HEIGHT,
        fraction(rec_secs, review.duration()),
    )
}

fn band(clip: &ClipEntry, review: &ReviewState, k: &Kit) -> El {
    let duration = review.duration();
    let start = fraction(clip.start_secs, duration);
    let end = fraction(clip.end_secs, duration);
    let band = el()
        .w_frac((end - start).max(0.0))
        .h(BAND_HEIGHT)
        .radius(3.0)
        .fill(k.c.text_base.alpha(BAND_ALPHA))
        .a11y_hidden();
    rune::row()
        .pin(Some(0.0), Some(0.0), Some(0.0), None)
        .h(HEIGHT)
        .center_y()
        .a11y_hidden()
        .child(el().w_frac(start).h(HEIGHT))
        .child(band)
}

fn thumb(size: f32, k: &Kit) -> El {
    el().size(size, size)
        .radius(size / 2.0)
        .fill(Color::WHITE)
        .border(1.0, k.c.border_strong)
        .a11y_hidden()
}

pub(super) fn timeline(review: &ReviewState, k: &Kit) -> El {
    let duration = review.duration();
    let progress = if duration > 0.0 {
        (review.time() / duration).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let track = rune::row()
        .key("timeline-track")
        .pin(
            Some(0.0),
            Some((HEIGHT - TRACK_HEIGHT) / 2.0),
            Some(0.0),
            None,
        )
        .h(TRACK_HEIGHT)
        .radius(TRACK_HEIGHT / 2.0)
        .fill(k.c.layer_03)
        .child(
            el().key("timeline-fill")
                .w_frac(progress)
                .h(TRACK_HEIGHT)
                .radius(TRACK_HEIGHT / 2.0)
                .fill(k.c.bg_accent)
                .instant_changes(),
        );
    let mut line = el()
        .key("timeline")
        .w_full()
        .h(HEIGHT)
        .hover_group()
        .on_drag(move |ui: &mut Ui, drag: &mut Drag, _| {
            if matches!(drag.phase, Phase::Down | Phase::Move) {
                ui.review_seek(drag_target(drag.x, drag.w, duration));
            }
        })
        .child(track);
    if review.source == Source::Game {
        for clip in &review.entry.meta.clips {
            line = line.child(band(clip, review, k));
        }
        for event in &review.entry.meta.events {
            line = line.child(marker(event, review, k));
        }
    }
    let hover = thumb(THUMB_HOVER, k)
        .key("timeline-thumb-hover")
        .group_opacity(0.0, 1.0);
    line.child(thumb_rail(
        thumb(THUMB, k).key("timeline-thumb"),
        THUMB,
        HEIGHT,
        progress,
    ))
    .child(thumb_rail(hover, THUMB_HOVER, HEIGHT, progress))
}
