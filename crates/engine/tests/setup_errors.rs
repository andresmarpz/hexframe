#![allow(clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::problem::Problem;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{Engine, EngineCmd, EngineStatus, FakeMediaSource, LeagueFactory, UiMsg};
use greyscreen_league::LeagueMsg;

const WAIT: Duration = Duration::from_secs(10);

fn silent_league() -> LeagueFactory {
    Box::new(|out: Sender<LeagueMsg>| Box::new(out))
}

#[test]
fn a_test_recording_into_an_unusable_folder_says_so_and_does_not_retry() {
    let root = tempfile::tempdir().expect("tempdir");
    let file = root.path().join("not-a-folder");
    std::fs::write(&file, b"x").expect("write");
    let (ui_tx, ui_rx) = unbounded();
    let engine = Engine::start(
        Settings {
            output_dir: file,
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::new()),
        silent_league(),
        Box::new(move |msg| {
            ui_tx.send(msg).ok();
        }),
    );
    engine
        .cmd
        .send(EngineCmd::StartTestRecording { secs: 1 })
        .expect("send");
    let mut error = None;
    let mut notice = None;
    while let Ok(msg) = ui_rx.recv_timeout(WAIT) {
        match msg {
            UiMsg::Status(EngineStatus::Error(text)) => {
                error = Some(text);
                break;
            }
            UiMsg::Notice { text, .. } => notice = Some(text),
            _ => {}
        }
    }
    let error = error.expect("an error status");
    assert_eq!(Problem::from_error(&error), Problem::FolderUnusable);
    let notice = notice.expect("a notice");
    assert!(
        notice.starts_with(Problem::FolderUnusable.title()),
        "{notice}"
    );
}

#[test]
fn a_test_recording_into_an_empty_folder_says_so() {
    let (ui_tx, ui_rx) = unbounded();
    let engine = Engine::start(
        Settings {
            output_dir: std::path::PathBuf::new(),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::new()),
        silent_league(),
        Box::new(move |msg| {
            ui_tx.send(msg).ok();
        }),
    );
    engine
        .cmd
        .send(EngineCmd::StartTestRecording { secs: 1 })
        .expect("send");
    let mut error = None;
    while let Ok(msg) = ui_rx.recv_timeout(WAIT) {
        if let UiMsg::Status(EngineStatus::Error(text)) = msg {
            error = Some(text);
            break;
        }
    }
    let error = error.expect("an error status");
    assert_eq!(Problem::from_error(&error), Problem::FolderUnusable);
}
