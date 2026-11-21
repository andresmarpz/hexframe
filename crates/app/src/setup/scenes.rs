use super::state::{Phase, SetupState};

type Scene = (&'static str, fn(&mut SetupState));

const SAMPLE_PATH: &str = r"C:\Users\you\AppData\Local\Programs\Greyscreen";

pub(super) fn preview() -> SetupState {
    SetupState::new(SAMPLE_PATH.to_owned(), None)
}

pub(super) const SCENES: &[Scene] = &[
    ("setup", |_| {}),
    ("setup-off", |s| s.start_with_windows = false),
    ("setup-closing", |s| s.phase = Phase::Closing),
    ("setup-installing", |s| s.phase = Phase::Copying),
    ("setup-done", |s| s.phase = Phase::Done),
    ("setup-error", |s| {
        s.phase = Phase::Failed(
            "Greyscreen is still running. Quit it from the tray, then run this file again.".into(),
        )
    }),
];
