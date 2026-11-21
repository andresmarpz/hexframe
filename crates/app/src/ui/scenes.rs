use greyscreen_core::keybinds::Action;
use greyscreen_core::update::UpdateState;
use greyscreen_engine::EngineStatus;
use rune::TextEdit;

use super::{ResultFilter, Screen, SettingsSection, Ui, preview, update};

type Scene = (&'static str, fn(&mut Ui));

pub(crate) const SCENES: &[Scene] = &[
    ("home", |ui| {
        ui.status = EngineStatus::Idle;
        ui.feed.clear();
    }),
    ("home-recording", |ui| ui.screen = Screen::Home),
    ("home-first-run", |ui| {
        ui.status = EngineStatus::Idle;
        ui.feed.clear();
        ui.recordings.clear();
    }),
    ("idle-off", |ui| {
        ui.status = EngineStatus::Idle;
        ui.settings.auto_record = false;
        ui.feed.clear();
    }),
    ("game-running-off", |ui| {
        ui.status = EngineStatus::GameRunning;
        ui.settings.auto_record = false;
        ui.feed.clear();
    }),
    ("last-game", |ui| {
        ui.status = EngineStatus::Idle;
        ui.feed.clear();
        ui.last_game = ui.recordings.first().cloned();
    }),
    ("waiting", |ui| {
        ui.status = EngineStatus::Waiting;
        ui.feed.clear();
    }),
    ("recording", |ui| ui.screen = Screen::Home),
    ("finalizing", |ui| ui.status = EngineStatus::Finalizing),
    ("error", |ui| {
        ui.status = EngineStatus::Error("The game window could not be captured.".into());
        ui.feed.clear();
    }),
    ("recordings", |ui| ui.screen = Screen::Recordings),
    ("recordings-empty", |ui| {
        ui.screen = Screen::Recordings;
        ui.recordings.clear();
    }),
    ("recordings-filtered", |ui| {
        ui.screen = Screen::Recordings;
        ui.filter.champion = TextEdit::new("ahri");
        ui.filter.result = ResultFilter::Victory;
    }),
    ("recordings-no-match", |ui| {
        ui.screen = Screen::Recordings;
        ui.filter.champion = TextEdit::new("zzz");
    }),
    ("recordings-delete", |ui| {
        ui.screen = Screen::Recordings;
        ui.filter.pending_delete = ui.recordings.get(1).map(|rec| rec.meta.id.clone());
    }),
    ("settings", |ui| open_sheet(ui, SettingsSection::Recording)),
    ("settings-resolution-open", |ui| {
        open_sheet(ui, SettingsSection::Recording);
        ui.resolution_open = true;
    }),
    ("settings-clips", |ui| {
        open_sheet(ui, SettingsSection::Clips)
    }),
    ("settings-clips-off", |ui| {
        ui.settings.clips.enabled = false;
        open_sheet(ui, SettingsSection::Clips);
    }),
    ("settings-storage", |ui| {
        ui.settings.storage_limit_gb = Some(50);
        open_sheet(ui, SettingsSection::Storage);
    }),
    ("settings-search", |ui| {
        open_sheet(ui, SettingsSection::Recording);
        ui.settings_search = TextEdit::new("fps");
    }),
    ("settings-about", |ui| {
        open_sheet(ui, SettingsSection::About)
    }),
    ("settings-update", |ui| {
        open_sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::Ready(update::preview_info()));
    }),
    ("settings-update-game", |ui| {
        open_sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::Ready(update::preview_info()));
        ui.update.install_when_idle = true;
    }),
    ("settings-update-manual", |ui| {
        open_sheet(ui, SettingsSection::About);
        ui.apply_update(UpdateState::AvailableManual(update::preview_info()));
    }),
    ("settings-keybinds", |ui| {
        open_sheet(ui, SettingsSection::Keybinds)
    }),
    ("settings-keybinds-capture", |ui| {
        open_sheet(ui, SettingsSection::Keybinds);
        ui.rebinding = Some(Action::PlayPause);
        ui.bind_hint = Some((
            Action::PlayPause,
            "F is already used by Toggle fullscreen.".into(),
        ));
    }),
    ("settings-keybinds-changed", |ui| {
        open_sheet(ui, SettingsSection::Keybinds);
        ui.settings.keybinds.insert("mute".into(), "N".into());
        ui.hotkey_errors = vec![(
            Action::SaveClip,
            "F9 is already used by another app. Choose a different key.".into(),
        )];
    }),
    ("review", |ui| {
        ui.open_review_entry(preview::review_entry());
    }),
    ("review-theater", |ui| {
        ui.open_review_entry(preview::review_entry());
        if let Some(review) = &mut ui.review {
            review.theater = true;
        }
    }),
];

fn open_sheet(ui: &mut Ui, section: SettingsSection) {
    ui.screen = Screen::Recordings;
    ui.open_settings(section);
}
