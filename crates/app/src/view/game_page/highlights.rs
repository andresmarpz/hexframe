use std::f32::consts::PI;

use greyscreen_core::library::ClipEntry;
use greyscreen_engine::RecordingEntry;
use rune::{Cursor, El, Icon, SurfaceFit, TextStyle, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Button, ButtonVariant, ControlSize, Kit};

use crate::ui::library::format::{clip_title, game_secs_at};
use crate::ui::{Screen, Ui, format_length};
use crate::view::TABULAR;
use crate::view::thumb::{ASPECT, clip_path, ready_image};

static ARROW: Icon = Icon {
    name: "greyscreen-arrow-right",
    view: 16.0,
    body: r#"<path d="M3.5 8H12.5M9 4.5L12.5 8L9 11.5" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

const SHOWN: usize = 4;
const GAP: f32 = 12.0;
const HEADER_HEIGHT: f32 = 28.0;
const CARD_RADIUS: f32 = 10.0;
const FADE_HEIGHT: f32 = 92.0;
const TEXT_INSET: f32 = 10.0;
const BADGE_INSET: f32 = 8.0;

pub(super) fn section(s: &Ui, rec: &RecordingEntry, k: &Kit) -> Option<El> {
    if rec.meta.clips.is_empty() {
        return None;
    }
    let shown = rec.meta.clips.len().min(SHOWN);
    let mut cards = row().w_full().gap(GAP);
    for (index, clip) in rec.meta.clips.iter().enumerate().take(SHOWN) {
        cards = cards.child(card(rec, index, clip, k));
    }
    for _ in shown..SHOWN {
        cards = cards.child(el().w(0.0).grow(1.0).a11y_hidden());
    }
    Some(
        col()
            .key("game-highlights")
            .w_full()
            .gap(8.0)
            .child(header(s, shown, k))
            .child(cards),
    )
}

fn header(s: &Ui, shown: usize, k: &Kit) -> El {
    let noun = if shown == 1 { "clip" } else { "clips" };
    let count = TextStyle {
        size: 13.0,
        line_height: 20.0,
        ..k.ty.ui.features(TABULAR)
    };
    let total = s.clip_count();
    let muted = k.c.text_muted;
    let all = Button::new("All clips")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .label_with(move |style, color| {
            row()
                .gap(6.0)
                .center_y()
                .child(text("All clips").font(style).color(color))
                .child(text(total.to_string()).font(count).color(muted))
                .child(rune::icon(&ARROW, 14.0).color(color))
        })
        .on_press(|ui: &mut Ui| ui.go(Screen::Clips))
        .build(k)
        .key("game-all-clips");
    row()
        .w_full()
        .h(HEADER_HEIGHT)
        .gap(8.0)
        .center_y()
        .child(text("Highlights").font(k.ty.ui_medium).color(k.c.text_base))
        .child(
            text(format!("· {shown} {noun}"))
                .font(count)
                .color(k.c.text_muted),
        )
        .child(el().grow(1.0))
        .child(all)
}

fn card(rec: &RecordingEntry, index: usize, clip: &ClipEntry, k: &Kit) -> El {
    let title = clip_title(&clip.label);
    let id = rec.meta.id.clone();
    let at = format_length(game_secs_at(rec, clip.start_secs) as u32);
    let length = format_length((clip.end_secs - clip.start_secs).max(0.0) as u32);
    let picture = match ready_image(&clip_path(rec, clip)) {
        Some(image) => el()
            .w_full()
            .h_full()
            .pixels_fit(image.pixels(), SurfaceFit::Fill),
        None => el().w_full().h_full(),
    };
    let fade = el()
        .pin(Some(0.0), None, Some(0.0), Some(0.0))
        .h(FADE_HEIGHT)
        .linear_gradient(PI, [(0.0, k.dark_a(0.0)), (1.0, k.dark_a(0.85))])
        .a11y_hidden();
    let badge = Badge::new(length)
        .variant(BadgeVariant::Neutral)
        .build(k)
        .pin(None, Some(BADGE_INSET), Some(BADGE_INSET), None);
    let caption = col()
        .pin(Some(TEXT_INSET), None, None, Some(TEXT_INSET))
        .gap(2.0)
        .child(
            text(title.clone())
                .font(TextStyle {
                    size: 13.0,
                    line_height: 16.0,
                    ..k.ty.ui_medium
                })
                .color(k.c.white),
        )
        .child(
            text(at)
                .font(TextStyle {
                    size: 12.0,
                    line_height: 16.0,
                    ..k.ty.ui.features(TABULAR)
                })
                .color(Kit::neutral(true).c.text_muted),
        );
    col().w(0.0).grow(1.0).min_w(0.0).child(
        el().key(format!("game-clip-{index}"))
            .role("button")
            .label(format!("Play {title} clip"))
            .w_full()
            .aspect_ratio(ASPECT)
            .radius(CARD_RADIUS)
            .clip_rounded(CARD_RADIUS)
            .fill(k.c.layer_02)
            .cursor(Cursor::Pointer)
            .on_press(move |ui: &mut Ui| ui.open_clip(id.clone(), index))
            .child(picture)
            .child(fade)
            .child(badge)
            .child(caption),
    )
}
