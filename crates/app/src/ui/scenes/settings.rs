use greyscreen_core::keybinds::Action;
use rune::TextEdit;

use super::{Scene, sheet};
use crate::ui::SettingsSection;

pub(super) const SCENES: &[Scene] = &[
    ("settings", |ui| sheet(ui, SettingsSection::Recording)),
    ("settings-resolution-open", |ui| {
        sheet(ui, SettingsSection::Recording);
        ui.resolution_open = true;
    }),
    ("settings-clips", |ui| sheet(ui, SettingsSection::Clips)),
    ("settings-clips-off", |ui| {
        ui.settings.clips.enabled = false;
        sheet(ui, SettingsSection::Clips);
    }),
    ("settings-storage", |ui| {
        ui.settings.storage_limit_gb = Some(50);
        sheet(ui, SettingsSection::Storage);
    }),
    ("settings-search", |ui| {
        sheet(ui, SettingsSection::Recording);
        ui.settings_search = TextEdit::new("fps");
    }),
    ("settings-about", |ui| sheet(ui, SettingsSection::About)),
    ("settings-keybinds", |ui| {
        sheet(ui, SettingsSection::Keybinds)
    }),
    ("settings-keybinds-capture", |ui| {
        sheet(ui, SettingsSection::Keybinds);
        ui.rebinding = Some(Action::PlayPause);
        ui.bind_hint = Some((
            Action::PlayPause,
            "F is already used by Toggle fullscreen.".into(),
        ));
    }),
    ("settings-keybinds-changed", |ui| {
        sheet(ui, SettingsSection::Keybinds);
        ui.settings.keybinds.insert("mute".into(), "N".into());
        ui.hotkey_errors = vec![(
            Action::SaveClip,
            "F9 is already used by another app. Choose a different key.".into(),
        )];
    }),
];
