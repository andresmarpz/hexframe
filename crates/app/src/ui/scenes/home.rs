use greyscreen_core::encoder::{CheckOutcome, SOFTWARE_LABEL, SOFTWARE_NOTICE};
use greyscreen_core::problem::{NO_FRAMES, Problem};
use greyscreen_engine::{EngineStatus, NoticeAction, NoticeLevel, RecordingEntry};

use super::{Scene, death, encoder, first_run, game, home};
use crate::ui::{Notice, Screen, TestRun, Ui, preview};

const SAMPLE_FAILURE: &str = "NVIDIA NVENC (hardware) on NVIDIA GeForce GT 1030: the encoder could not start (0x8000FFFF)";

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
    ("home-no-encoder", |ui| {
        home(ui);
        ui.last_game = ui.recordings.first().cloned();
        ui.encoder = encoder(CheckOutcome::Failed, "");
        if let Some(check) = ui.encoder.as_mut() {
            check.first_failure = SAMPLE_FAILURE.into();
        }
    }),
    ("software-encoder-notice", |ui| {
        home(ui);
        ui.last_game = ui.recordings.first().cloned();
        ui.encoder = encoder(CheckOutcome::Software, SOFTWARE_LABEL);
        let notice = Notice {
            level: NoticeLevel::Warn,
            text: SOFTWARE_NOTICE.to_owned(),
            action: Some(NoticeAction::Link(Problem::NoEncoder.help_url())),
        };
        ui.notice = Some((notice, f64::MAX));
    }),
    ("coach-demo", |ui| {
        home(ui);
        death::browse(ui);
    }),
];
