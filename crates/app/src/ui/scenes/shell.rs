use super::{Scene, home};
use crate::ui::{Screen, preview};

pub(super) const SCENES: &[Scene] = &[
    ("top-bar", |ui| {
        home(ui);
        ui.recordings = preview::demo_recordings();
    }),
    ("top-bar-clips", |ui| {
        home(ui);
        ui.recordings = preview::demo_recordings();
        ui.go(Screen::Clips);
    }),
];
