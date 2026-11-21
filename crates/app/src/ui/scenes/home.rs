use greyscreen_core::problem::NO_FRAMES;
use greyscreen_engine::{EngineStatus, RecordingEntry};

use super::{Scene, death, first_run, game, home};
use crate::ui::{Screen, TestRun, Ui, preview};

fn test_recording() -> RecordingEntry {
    let mut entry = preview::demo_recordings().remove(0);
    entry.meta.duration_secs = 10.0;
    entry
}

fn demo(ui: &mut Ui) {
    home(ui);
    ui.recordings = preview::demo_recordings();
}

pub(super) const SCENES: &[Scene] = &[
    ("home", |ui| {
        home(ui);
        ui.last_game = ui.recordings.first().cloned();
    }),
    ("demo", demo),
    ("home-first-run", |ui| {
        first_run(ui);
        ui.settings.first_run_done = false;
        ui.go(Screen::Home);
    }),
    ("first-run-testing", |ui| {
        first_run(ui);
        ui.settings.first_run_done = false;
        ui.first_test = TestRun::Running;
        ui.go(Screen::Home);
    }),
    ("first-run-works", |ui| {
        first_run(ui);
        ui.settings.first_run_done = false;
        ui.first_test = TestRun::Passed(Box::new(test_recording()));
        ui.go(Screen::Home);
    }),
    ("first-run-failed", |ui| {
        first_run(ui);
        ui.settings.first_run_done = false;
        ui.first_test = TestRun::Failed;
        ui.status = EngineStatus::Error(NO_FRAMES.to_owned());
        ui.go(Screen::Home);
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
    ("coach-demo", |ui| {
        home(ui);
        death::browse(ui);
    }),
];
