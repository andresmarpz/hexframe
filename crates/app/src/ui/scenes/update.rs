use greyscreen_core::update::UpdateState;

use super::{Scene, docked, home, preview, sheet};
use crate::ui::SettingsSection;

pub(super) const SCENES: &[Scene] = &[
    ("update-ready", |ui| {
        home(ui);
        ui.apply_update(UpdateState::Ready(preview::update_info()));
    }),
    ("update-ready-dock", |ui| {
        docked(ui);
        ui.apply_update(UpdateState::Ready(preview::update_info()));
    }),
    ("settings-update", |ui| {
        sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::Ready(preview::update_info()));
    }),
    ("settings-update-game", |ui| {
        sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::Ready(preview::update_info()));
        ui.update.install_when_idle = true;
    }),
    ("settings-update-checking", |ui| {
        sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::Checking);
    }),
    ("settings-update-manual", |ui| {
        sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::AvailableManual(preview::update_info()));
    }),
];
