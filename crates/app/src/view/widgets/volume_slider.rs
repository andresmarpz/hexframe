use rune::{Color, El, el, row};
use rune_kit::Kit;

const SLIDER_HEIGHT: f32 = 20.0;
const TRACK_HEIGHT: f32 = 4.0;
const THUMB: f32 = 12.0;
pub(crate) const VOLUME_STEP: f32 = 0.05;

pub(crate) fn percent(volume: f32) -> u32 {
    (volume * 100.0).round() as u32
}

pub(crate) fn volume_at(x: f32, width: f32) -> f32 {
    let travel = width - THUMB;
    if travel <= 0.0 {
        return 0.0;
    }
    let value = ((x - THUMB / 2.0) / travel).clamp(0.0, 1.0);
    (value * 100.0).round() / 100.0
}

pub(crate) fn volume_slider(k: &Kit, name: &str, value: f32) -> El {
    let track = el()
        .key(format!("{name}-track"))
        .pin(
            Some(0.0),
            Some((SLIDER_HEIGHT - TRACK_HEIGHT) / 2.0),
            Some(0.0),
            None,
        )
        .h(TRACK_HEIGHT)
        .radius(TRACK_HEIGHT / 2.0)
        .fill(k.c.layer_03);
    let knob = row()
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .pr(THUMB)
        .center_y()
        .instant_changes()
        .child(
            el().key(format!("{name}-fill"))
                .w_frac(value)
                .h(TRACK_HEIGHT)
                .radius(TRACK_HEIGHT / 2.0)
                .fill(k.c.bg_accent)
                .instant_changes(),
        )
        .child(
            el().key(format!("{name}-thumb"))
                .size(THUMB, THUMB)
                .radius(THUMB / 2.0)
                .fill(Color::WHITE)
                .border(1.0, k.c.border_strong),
        );
    el().key(format!("{name}-slider"))
        .label("Volume")
        .range_value(f64::from(percent(value)), 0.0, 100.0)
        .range_step(f64::from(percent(VOLUME_STEP)))
        .grow(1.0)
        .h(SLIDER_HEIGHT)
        .child(track)
        .child(knob)
}
