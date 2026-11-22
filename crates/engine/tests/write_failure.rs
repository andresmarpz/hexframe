#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use greyscreen_core::problem::DISK_FULL_NOTICE;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, FakeMediaSource, NoticeLevel, UiMsg, set_write_limit};

use common::{WAIT, assert_playable_from_key_frame, recording_since, start_idle, temp_dir};

const WRITE_LIMIT: u64 = 80_000;
const HEAR_WITHIN: Duration = Duration::from_secs(6);

#[test]
fn a_full_disk_stops_the_game_video_at_once_and_clips_keep_saving() {
    set_write_limit(Some(WRITE_LIMIT));
    let settings = Settings {
        output_dir: temp_dir("write-failure"),
        ..Settings::default()
    };
    let harness = start_idle(settings, Arc::new(FakeMediaSource::new()));
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 5 }).ok();
    recording_since(&harness.ui);
    let began = Instant::now();

    let mut notices = Vec::new();
    let mut heard_at = None;
    while heard_at.is_none() {
        let msg = harness
            .ui
            .recv_timeout(HEAR_WITHIN)
            .expect("disk full notice");
        if let UiMsg::Notice { level, text } = msg {
            heard_at = (text == DISK_FULL_NOTICE).then(|| began.elapsed());
            notices.push((level, text));
        }
    }
    assert!(heard_at.expect("heard") < HEAR_WITHIN);

    cmd.send(EngineCmd::SaveClipNow).ok();
    let mut clip = None;
    let mut saved = None;
    let deadline = Instant::now() + WAIT;
    while saved.is_none() && Instant::now() < deadline {
        match harness.ui.recv_timeout(WAIT).expect("ui message") {
            UiMsg::ClipSaved(entry) => clip = Some(entry),
            UiMsg::RecordingSaved(entry) => saved = Some(entry),
            UiMsg::Notice { level, text } => notices.push((level, text)),
            _ => {}
        }
    }
    set_write_limit(None);

    let clip = clip.expect("clip saved after the write failure");
    assert_playable_from_key_frame(&clip.folder.join(&clip.clip.file));
    let saved = saved.expect("recording saved");
    let errors: Vec<_> = notices
        .iter()
        .filter(|(level, _)| *level == NoticeLevel::Error)
        .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let health = saved.meta.health.expect("health");
    assert!(health.video_stopped_secs.is_some());
    assert_eq!(saved.meta.video, "");

    thread::sleep(Duration::from_millis(100));
    let part = saved.folder.join("game.mp4.part");
    assert!(part.is_file());
    let recovered = greyscreen_mp4::recover::recover(&part).expect("recover the written prefix");
    assert!(recovered.is_file());
}
