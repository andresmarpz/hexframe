use greyscreen_core::highlight::{Tone, describe_label, dominant_label};
use greyscreen_core::library::EventEntry;
use rune::{Color, Drag, El, Phase, el};
use rune_kit::Kit;

use super::card::hoverable;
use crate::ui::{ReviewState, Source, Ui, seek_target, tone_color};

const HEIGHT: f32 = 28.0;
const TRACK_HEIGHT: f32 = 4.0;
const MARKER_WIDTH: f32 = 8.0;
const MARKER_HEIGHT: f32 = 18.0;

pub(super) fn event_color(k: &Kit, highlights: &[String]) -> Color {
    let tone = dominant_label(highlights.iter().map(String::as_str))
        .map_or(Tone::Neutral, |label| describe_label(label).tone);
    tone_color(k, tone)
}

pub(super) fn marker_x(rec_secs: f64, duration: f64, width: f32) -> f32 {
    if duration <= 0.0 {
        return 0.0;
    }
    (rec_secs / duration).clamp(0.0, 1.0) as f32 * width
}

pub(super) fn drag_target(x: f32, width: f32, duration: f64) -> f64 {
    if width <= 0.0 {
        return 0.0;
    }
    f64::from((x / width).clamp(0.0, 1.0)) * duration
}

fn marker(event: &EventEntry, review: &ReviewState, k: &Kit) -> El {
    let x = marker_x(event.rec_secs, review.duration(), review.track_width) - MARKER_WIDTH / 2.0;
    let x = x.clamp(0.0, (review.track_width - MARKER_WIDTH).max(0.0));
    let rec_secs = event.rec_secs;
    let key = format!("marker-{}", event.id);
    let marker = el()
        .key(key.clone())
        .size(MARKER_WIDTH, MARKER_HEIGHT)
        .absolute(x, (HEIGHT - MARKER_HEIGHT) / 2.0)
        .radius(2.0)
        .fill(event_color(k, &event.highlights))
        .on_press(move |ui: &mut Ui| ui.review_seek(seek_target(rec_secs)));
    hoverable(marker, event.id, key)
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
                .fill(k.c.bg_accent),
        );
    let mut line = el()
        .key("timeline")
        .grow(1.0)
        .min_w(0.0)
        .h(HEIGHT)
        .on_layout(|ui: &mut Ui, rect: [f32; 4]| {
            if let Some(review) = &mut ui.review {
                review.track_width = rect[2];
            }
        })
        .on_drag(move |ui: &mut Ui, drag: &mut Drag, _| {
            if matches!(drag.phase, Phase::Down | Phase::Move) {
                ui.review_seek(drag_target(drag.x, drag.w, duration));
            }
        })
        .child(track);
    if review.source == Source::Game {
        for event in &review.entry.meta.events {
            line = line.child(marker(event, review, k));
        }
    }
    line
}
