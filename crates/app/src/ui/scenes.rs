use greyscreen_core::keybinds::Action;
use greyscreen_core::update::UpdateState;
use greyscreen_engine::EngineStatus;
use rune::TextEdit;

use super::{SettingsSection, Ui, preview, update};

type Scene = (&'static str, fn(&mut Ui));

fn idle(ui: &mut Ui) {
    ui.status = EngineStatus::Idle;
    ui.feed.clear();
}

fn first_run(ui: &mut Ui) {
    idle(ui);
    ui.recordings.clear();
}

fn game(ui: &mut Ui) {
    idle(ui);
    ui.open_review_entry(preview::review_entry());
}

fn sheet(ui: &mut Ui, section: SettingsSection) {
    game(ui);
    ui.open_settings(section);
}

fn demo(ui: &mut Ui) {
    idle(ui);
    ui.recordings = preview::demo_recordings();
}

pub(crate) const SCENES: &[Scene] = &[
    ("home", idle),
    ("demo", demo),
    ("home-first-run", first_run),
    ("idle-off", |ui| {
        idle(ui);
        ui.settings.auto_record = false;
    }),
    ("game-running-off", |ui| {
        idle(ui);
        ui.status = EngineStatus::GameRunning;
        ui.settings.auto_record = false;
    }),
    ("last-game", |ui| {
        idle(ui);
        ui.last_game = ui.recordings.first().cloned();
    }),
    ("waiting", |ui| {
        idle(ui);
        ui.status = EngineStatus::Waiting;
    }),
    ("recording", |ui| ui.select_live()),
    ("recording-game", |ui| {
        game(ui);
        ui.status = Ui::preview().status;
        ui.elapsed_secs = 12 * 60 + 34;
    }),
    ("finalizing", |ui| {
        ui.status = EngineStatus::Finalizing;
        ui.select_live();
    }),
    ("error", |ui| {
        idle(ui);
        ui.status = EngineStatus::Error("The game window could not be captured.".into());
    }),
    ("recordings", game),
    ("recordings-empty", first_run),
    ("recordings-filtered", |ui| {
        idle(ui);
        ui.filter.champion = TextEdit::new("ahri");
    }),
    ("recordings-no-match", |ui| {
        idle(ui);
        ui.filter.champion = TextEdit::new("zzz");
    }),
    ("recordings-delete", |ui| {
        game(ui);
        ui.filter.pending_delete = ui.review.as_ref().map(|r| r.entry.meta.id.clone());
    }),
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
    ("settings-update", |ui| {
        sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::Ready(update::preview_info()));
    }),
    ("settings-update-game", |ui| {
        sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::Ready(update::preview_info()));
        ui.update.install_when_idle = true;
    }),
    ("settings-update-manual", |ui| {
        sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::AvailableManual(update::preview_info()));
    }),
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
    ("review", game),
    ("review-theater", |ui| {
        game(ui);
        if let Some(review) = &mut ui.review {
            review.theater = true;
        }
    }),
];
