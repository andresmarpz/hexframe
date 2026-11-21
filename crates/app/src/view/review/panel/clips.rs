use crate::qa::reveal;
use greyscreen_core::library::ClipEntry;
use rune::{El, col, row, text};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit, glyphs};

use super::{faded_scroll, mark_active, muted_note};
use crate::ui::library::format::clip_title;
use crate::ui::{ReviewState, Ui, format_clock};
use crate::view::review::tabular;
use crate::view::widgets::{EXPORT, thumb};

const CLIP_ROW_HEIGHT: f32 = 52.0;
const CLIP_THUMB_WIDTH: f32 = 64.0;
const CLIP_THUMB_HEIGHT: f32 = 36.0;

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
    let id = review.entry.meta.id.clone();
    let export = IconButton::new(&EXPORT)
        .label("Export clip")
        .help("Export…")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(move |ui: &mut Ui| ui.open_export_clip(&id, index))
        .build(k)
        .key(format!("export-clip-{index}"))
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
        .child(export)
        .child(show);
    k.interactive(mark_active(
        line,
        active.then(|| format!("clip-active-{index}")),
        k,
    ))
}

pub(super) fn clips_list(review: &ReviewState, k: &Kit) -> El {
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
