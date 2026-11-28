use rune::{El, el, row};
use rune_kit::Kit;

use super::recorder_dock;
use crate::ui::{Detail, Ui};

pub(super) const DOCK_HEIGHT: f32 = 48.0;
const BOTTOM_GAP: f32 = 16.0;
const HOVER_ZONE: f32 = 104.0;
const FRAME: f32 = 8.0;
const REVIEW_PADDING: f32 = 16.0;
pub(in crate::view) const CLEARANCE: f32 = DOCK_HEIGHT + BOTTOM_GAP * 2.0;
pub(in crate::view) const REVIEW_ROOM: f32 = CLEARANCE - FRAME - REVIEW_PADDING;

pub(in crate::view) fn layer(s: &Ui, k: &Kit) -> Option<El> {
    s.dock.visible.then(|| {
        row()
            .key("dock-layer")
            .pin(Some(0.0), None, Some(0.0), Some(BOTTOM_GAP))
            .justify_center()
            .child(recorder_dock::dock(s, k))
    })
}

pub(in crate::view) fn overlay(s: &Ui, k: &Kit, layer: &mut Vec<El>) {
    if s.dock.visible {
        recorder_dock::overlay(s, k, layer);
    }
}

pub(in crate::view) fn hover_zone(s: &Ui) -> Option<El> {
    (s.detail() == Detail::Game).then(|| {
        el().key("dock-zone")
            .pin(Some(0.0), None, Some(0.0), Some(0.0))
            .h(HOVER_ZONE)
            .on_hover_change(|ui: &mut Ui, inside, tx| ui.dock_hover(inside, tx))
            .on_pointer_move_local(|ui: &mut Ui, _, tx| ui.wake_dock(tx.clock()))
            .a11y_hidden()
    })
}
