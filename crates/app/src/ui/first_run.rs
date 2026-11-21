use greyscreen_engine::{EngineCmd, EngineStatus, RecordingEntry};
use rune::{FileDialogOptions, Tx};

use super::{TEST_RECORDING_SECS, Ui};

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum TestRun {
    #[default]
    Idle,
    Running,
    Passed(Box<RecordingEntry>),
    Failed,
}

impl Ui {
    pub(crate) fn run_first_test(&mut self) {
        self.first_test = TestRun::Running;
        self.send(EngineCmd::StartTestRecording {
            secs: TEST_RECORDING_SECS,
        });
    }

    pub(crate) fn finish_first_run(&mut self) {
        self.first_test = TestRun::Idle;
        self.change(|s| s.first_run_done = true);
    }

    pub(crate) fn choose_folder(&self, tx: &mut Tx) {
        let options = FileDialogOptions {
            title: "Choose the recordings folder".into(),
            directory: Some(self.settings.output_dir.clone()),
            ..Default::default()
        };
        tx.open_folder(options, |ui: &mut Ui, answer, _| {
            if let Ok(Some(dir)) = answer {
                ui.change(|s| s.output_dir = dir);
            }
        });
    }

    pub(super) fn first_test_saved(&mut self, entry: &RecordingEntry) {
        if self.first_test == TestRun::Running {
            self.first_test = TestRun::Passed(Box::new(entry.clone()));
        }
    }

    pub(super) fn first_test_status(&mut self, status: &EngineStatus) {
        if self.first_test == TestRun::Running && matches!(status, EngineStatus::Error(_)) {
            self.first_test = TestRun::Failed;
        }
    }
}
