use super::{Scene, docked};
use crate::ui::{Screen, Ui, preview};

pub(super) const SCENES: &[Scene] = &[
    ("dock", |ui| {
        docked(ui);
        ui.go(Screen::Home);
    }),
    ("dock-recording", |ui| {
        docked(ui);
        ui.status = Ui::preview().status;
        ui.elapsed_secs = 12 * 60 + 34;
        ui.go(Screen::Clips);
    }),
    ("dock-games", |ui| {
        docked(ui);
        ui.open_review_entry(preview::review_entry());
    }),
    ("dock-pick", |ui| {
        docked(ui);
        ui.go(Screen::Games);
    }),
];
