#![allow(clippy::expect_used)]

mod common;

use std::fs;
use std::sync::Arc;

use greyscreen_core::library;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, EngineStatus, FakeMediaSource, UiMsg};

use common::{leftover_part, library_message, media, start_idle, temp_dir, wait_for};

#[test]
fn recovery_sets_the_video_when_the_sidecar_has_a_duration_but_no_video() {
    let media = media(120);
    let output = temp_dir("recover-video");
    let folder = leftover_part(&output, "2025-01-04_00-00-00_Ahri_CLASSIC", &media, true);
    let mut meta = library::read_meta(&folder).expect("sidecar");
    meta.video = String::new();
    meta.duration_secs = 12.0;
    library::write_meta(&folder, &meta).expect("sidecar");

    let harness = start_idle(
        Settings {
            output_dir: output,
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
    );
    let entries = library_message(&harness);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].meta.video, "game.mp4");
    assert!((entries[0].meta.duration_secs - 12.0).abs() < 1e-9);
    assert!(folder.join("game.mp4").is_file());
}

#[test]
fn a_failed_final_rename_is_repaired_by_recovery() {
    let output = temp_dir("rename-fails");
    let settings = Settings {
        output_dir: output.clone(),
        ..Settings::default()
    };
    let blocked = output.clone();
    let tap: common::Tap = Box::new(move |msg| {
        if matches!(msg, UiMsg::Status(EngineStatus::Recording { .. }))
            && let Some(folder) = fs::read_dir(&blocked)
                .ok()
                .and_then(|mut dir| dir.next())
                .and_then(Result::ok)
        {
            fs::create_dir_all(folder.path().join("game.mp4/blocker")).ok();
        }
    });
    let mut harness = common::start_engine(
        settings.clone(),
        Arc::new(FakeMediaSource::new().with_speed(20)),
        common::no_league(),
        Some(tap),
    );
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 1 }).ok();
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    assert_eq!(saved.meta.video, "");
    assert!(saved.folder.join("game.mp4.part").is_file());
    drop(cmd);
    harness.engine.take();

    fs::remove_dir_all(saved.folder.join("game.mp4")).expect("unblock");
    let second = start_idle(settings, Arc::new(FakeMediaSource::fast()));
    let entries = library_message(&second);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].meta.video, "game.mp4");
    assert!(
        (1.0..5.0).contains(&entries[0].meta.duration_secs),
        "{}",
        entries[0].meta.duration_secs
    );
    assert!(saved.folder.join("game.mp4").is_file());
}
