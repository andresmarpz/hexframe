use super::{Scene, home};
use crate::view::{icon_sheet, timeline_sheet};

pub(super) const SCENES: &[Scene] = &[
    ("icons", |ui| {
        home(ui);
        icon_sheet::show();
    }),
    ("timeline-boxes", |ui| {
        home(ui);
        timeline_sheet::show();
    }),
];
