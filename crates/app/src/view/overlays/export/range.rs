use rune::{Color, Drag, El, Phase, col, el, row, text};
use rune_kit::Kit;

use crate::ui::{ExportUi, RangeHandle, Ui, format_clock};
use crate::view::TABULAR;

const HEIGHT: f32 = 28.0;
const TRACK_HEIGHT: f32 = 4.0;
const BAND_HEIGHT: f32 = 10.0;
const THUMB: f32 = 14.0;
const BAND_ALPHA: f32 = 0.22;

pub(crate) fn pointer_fraction(x: f32, width: f32) -> f32 {
    let travel = width - THUMB;
    if travel <= 0.0 {
        return 0.0;
    }
    ((x - THUMB / 2.0) / travel).clamp(0.0, 1.0)
}

pub(crate) fn nearest_handle(pointer: f32, start: f32, end: f32) -> RangeHandle {
    if (pointer - start).abs() <= (pointer - end).abs() {
        RangeHandle::Start
    } else {
        RangeHandle::End
    }
}

fn rail(item: El, progress: f32) -> El {
    row()
        .pin(Some(0.0), Some(0.0), Some(0.0), None)
        .h(HEIGHT)
        .px(THUMB / 2.0)
        .center_y()
        .child(el().w_frac(progress).h(HEIGHT))
        .child(item)
        .instant_changes()
        .a11y_hidden()
}

fn handle(key: &'static str, progress: f32, k: &Kit) -> El {
    rail(
        el().key(key)
            .size(THUMB, THUMB)
            .ml(-THUMB / 2.0)
            .radius(THUMB / 2.0)
            .fill(Color::WHITE)
            .border(1.0, k.c.border_strong),
        progress,
    )
}

fn span(from: f32, to: f32, height: f32, key: &'static str, fill: Color, radius: f32) -> El {
    row()
        .pin(Some(0.0), Some(0.0), Some(0.0), None)
        .h(HEIGHT)
        .center_y()
        .a11y_hidden()
        .child(el().w_frac(from).h(HEIGHT))
        .child(
            el().key(key)
                .w_frac((to - from).max(0.0))
                .h(height)
                .radius(radius)
                .fill(fill),
        )
}

pub(super) fn bar(export: &ExportUi, k: &Kit) -> El {
    let (start, end) = export.range_fractions();
    let (clip_start, clip_end) = export.default_fractions();
    let track = el()
        .pin(
            Some(0.0),
            Some((HEIGHT - TRACK_HEIGHT) / 2.0),
            Some(0.0),
            None,
        )
        .h(TRACK_HEIGHT)
        .radius(TRACK_HEIGHT / 2.0)
        .fill(k.c.layer_03)
        .a11y_hidden();
    el().key("export-range")
        .label("Trim range")
        .w_full()
        .h(HEIGHT)
        .on_drag(|ui: &mut Ui, drag: &mut Drag, _| {
            let pointer = pointer_fraction(drag.x, drag.w);
            if drag.phase == Phase::Down {
                let (start, end) = ui.export.range_fractions();
                drag.data = match nearest_handle(pointer, start, end) {
                    RangeHandle::Start => 0.0,
                    RangeHandle::End => 1.0,
                };
            }
            if matches!(drag.phase, Phase::Down | Phase::Move) {
                let handle = if drag.data < 0.5 {
                    RangeHandle::Start
                } else {
                    RangeHandle::End
                };
                ui.drag_export_handle(handle, pointer);
            }
        })
        .child(track)
        .child(span(
            clip_start,
            clip_end,
            BAND_HEIGHT,
            "export-clip-band",
            k.c.text_base.alpha(BAND_ALPHA),
            3.0,
        ))
        .child(span(
            start,
            end,
            TRACK_HEIGHT,
            "export-selection",
            k.c.bg_accent,
            TRACK_HEIGHT / 2.0,
        ))
        .child(handle("export-start-handle", start, k))
        .child(handle("export-end-handle", end, k))
}

pub(super) fn readout(export: &ExportUi, k: &Kit) -> El {
    let font = k.ty.body.features(TABULAR);
    let seconds = export.range.duration().round() as u32;
    row()
        .key("export-readout")
        .w_full()
        .center_y()
        .child(
            text(format_clock(export.range.start as u32))
                .key("export-from")
                .font(font)
                .color(k.c.text_base),
        )
        .child(col().grow(1.0).a11y_hidden())
        .child(
            text(format!("{seconds} s"))
                .key("export-length")
                .font(font)
                .color(k.c.text_muted),
        )
        .child(col().grow(1.0).a11y_hidden())
        .child(
            text(format_clock(export.range.end as u32))
                .key("export-to")
                .font(font)
                .color(k.c.text_base),
        )
}
