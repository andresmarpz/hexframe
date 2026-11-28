use greyscreen_engine::EngineStatus;

use super::{Scene, home};
use crate::ui::{Ui, preview};

fn dock(ui: &mut Ui) {
    home(ui);
    ui.recordings = preview::demo_recordings();
    ui.settings.storage_limit_gb = Some(250);
}

pub(super) const SCENES: &[Scene] = &[
    ("dock-ready", |ui| {
        dock(ui);
        ui.settings.auto_record = true;
    }),
    ("dock-recording", |ui| {
        dock(ui);
        ui.status = preview::ui().status;
        ui.elapsed_secs = 12 * 60 + 34;
    }),
    ("dock-paused", |ui| {
        dock(ui);
        ui.settings.auto_record = false;
    }),
    ("dock-storage", |ui| {
        dock(ui);
        ui.settings.auto_record = true;
        ui.dock.storage_open = true;
    }),
    ("dock-storage-unlimited", |ui| {
        dock(ui);
        ui.settings.storage_limit_gb = None;
        ui.dock.storage_open = true;
    }),
    ("dock-waiting", |ui| {
        dock(ui);
        ui.status = EngineStatus::Waiting;
    }),
];
