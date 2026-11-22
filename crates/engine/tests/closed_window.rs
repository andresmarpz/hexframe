#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, FakeMediaSource, NoticeLevel, UiMsg};

use common::{WAIT, assert_playable_from_key_frame, recording_since, start_idle, temp_dir};

const CLOSED_REASON: &str = "the captured window closed";

#[test]
fn a_window_closed_mid_recording_finalises_the_file_with_the_closed_window_reason() {
    let settings = Settings {
        output_dir: temp_dir("closed-window"),
        ..Settings::default()
    };
    let media = FakeMediaSource::new().failing_after(Duration::from_secs(2), CLOSED_REASON);
    let harness = start_idle(settings, Arc::new(media));
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 30 }).ok();
    recording_since(&harness.ui);

    let mut errors = Vec::new();
    let saved = loop {
        match harness.ui.recv_timeout(WAIT).expect("recording saved") {
            UiMsg::RecordingSaved(entry) => break entry,
            UiMsg::Notice {
                level: NoticeLevel::Error,
                text,
            } => errors.push(text),
            _ => {}
        }
    };

    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("League window not found"),
        "{errors:?}"
    );
    let video = saved.folder.join(&saved.meta.video);
    assert!(video.is_file());
    assert!(!saved.folder.join("game.mp4.part").exists());
    assert_playable_from_key_frame(&video);
}
