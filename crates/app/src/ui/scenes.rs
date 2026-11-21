use greyscreen_core::keybinds::Action;
use greyscreen_core::settings::NavStyle;
use greyscreen_core::update::UpdateState;
use greyscreen_engine::EngineStatus;
use rune::{PointerPress, TextEdit};

use super::{PanelTab, Screen, SettingsSection, Ui, death_review, preview, update};

type Scene = (&'static str, fn(&mut Ui));

fn idle(ui: &mut Ui) {
    ui.status = EngineStatus::Idle;
    ui.feed.clear();
}

fn home(ui: &mut Ui) {
    idle(ui);
    ui.go(Screen::Home);
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

fn docked(ui: &mut Ui) {
    idle(ui);
    ui.settings.nav_style = NavStyle::Dock;
}

fn demo(ui: &mut Ui) {
    home(ui);
    ui.recordings = preview::demo_recordings();
}

pub(crate) const SCENES: &[Scene] = &[
    ("home", |ui| {
        home(ui);
        ui.last_game = ui.recordings.first().cloned();
    }),
    ("demo", demo),
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
    ("home-first-run", |ui| {
        first_run(ui);
        ui.go(Screen::Home);
    }),
    ("games-pick", idle),
    ("clips", |ui| {
        idle(ui);
        ui.go(Screen::Clips);
    }),
    ("clips-empty", |ui| {
        first_run(ui);
        ui.go(Screen::Clips);
    }),
    ("idle-off", |ui| {
        home(ui);
        ui.settings.auto_record = false;
    }),
    ("game-running-off", |ui| {
        home(ui);
        ui.status = EngineStatus::GameRunning;
        ui.settings.auto_record = false;
    }),
    ("last-game", |ui| {
        home(ui);
        ui.last_game = ui.recordings.first().cloned();
    }),
    ("waiting", |ui| {
        home(ui);
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
        home(ui);
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
    ("update-ready", |ui| {
        home(ui);
        ui.apply_update(UpdateState::Ready(update::preview_info()));
    }),
    ("update-ready-dock", |ui| {
        docked(ui);
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
    ("coach-demo", |ui| {
        home(ui);
        death_review::scene_browse(ui);
    }),
    ("row-menu", |ui| {
        idle(ui);
        if let Some(id) = ui.recordings.first().map(|rec| rec.meta.id.clone()) {
            ui.open_row_menu(id, PointerPress { x: 140.0, y: 190.0 });
        }
    }),
    ("review", game),
    ("review-clips", |ui| {
        game(ui);
        if let Some(review) = &mut ui.review {
            review.tab = PanelTab::Clips;
        }
    }),
    ("death-review-start", |ui| {
        idle(ui);
        death_review::scene_demo(ui);
    }),
    ("death-review", |ui| {
        idle(ui);
        death_review::scene_mid(ui);
    }),
    ("death-review-summary", |ui| {
        idle(ui);
        death_review::scene_summary(ui);
    }),
    ("review-theater", |ui| {
        game(ui);
        if let Some(review) = &mut ui.review {
            review.theater = true;
        }
    }),
];
