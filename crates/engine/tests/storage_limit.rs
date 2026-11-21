#![allow(clippy::expect_used)]

mod common;

use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;

use greyscreen_core::game::{GameInfo, GameResult, Player, Team};
use greyscreen_core::library::{
    self, ClipEntry, GameRecord, META_FILE, META_VERSION, RecordingMeta, VideoInfo,
};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, FakeMediaSource, NoticeLevel, UiMsg};

use common::{start_idle, temp_dir, wait_for};

const GB: u64 = 1_000_000_000;

fn old_game(output: &Path, id: &str, day: u32, video_bytes: u64) {
    let player = Player {
        name: "Me".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    };
    let folder = output.join(id);
    fs::create_dir_all(folder.join("clips")).expect("folder");
    let meta = RecordingMeta {
        version: META_VERSION,
        id: id.into(),
        started_at: format!("2026-10-{day:02}T12:00:00[UTC]")
            .parse()
            .expect("date"),
        duration_secs: 1800.0,
        video: "game.mp4".into(),
        game: GameRecord {
            info: GameInfo {
                mode: "CLASSIC".into(),
                map: Some(11),
                me: player.clone(),
                players: vec![player],
            },
            result: GameResult::Win,
        },
        events: Vec::new(),
        clips: vec![ClipEntry {
            file: "clips/01-00_Kill.mp4".into(),
            start_secs: 60.0,
            end_secs: 80.0,
            label: "Kill".into(),
        }],
        encoder: "test".into(),
        video_format: VideoInfo {
            width: 1920,
            height: 1080,
            fps: 60,
        },
        review: Default::default(),
        health: None,
        recovered: false,
    };
    library::write_meta(&folder, &meta).expect("meta");
    fs::write(folder.join("clips/01-00_Kill.mp4"), b"clip").expect("clip");
    sparse_file(&folder.join("game.mp4"), video_bytes).expect("sparse video");
}

fn sparse_file(path: &Path, bytes: u64) -> std::io::Result<()> {
    File::create(path)?;
    #[cfg(windows)]
    std::process::Command::new("fsutil")
        .args(["sparse", "setflag"])
        .arg(path)
        .output()?;
    fs::OpenOptions::new()
        .write(true)
        .open(path)?
        .set_len(bytes)
}

fn limited(tag: &str, gb: u32) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        storage_limit_gb: Some(gb),
        ..Settings::default()
    }
}

#[test]
fn startup_trims_the_oldest_videos_and_keeps_the_clips() {
    let settings = limited("limit-startup", 50);
    old_game(&settings.output_dir, "older", 1, 30 * GB);
    old_game(&settings.output_dir, "newer", 2, 30 * GB);
    let output = settings.output_dir.clone();
    let harness = start_idle(settings, Arc::new(FakeMediaSource::new()));
    let notice = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice {
            level: NoticeLevel::Info,
            text,
        } => Some(text.clone()),
        _ => None,
    })
    .expect("cleanup notice");
    assert!(notice.contains("deleted 1 old game video"), "{notice}");
    assert!(!output.join("older/game.mp4").exists());
    assert!(output.join("older").join(META_FILE).is_file());
    assert!(output.join("older/clips/01-00_Kill.mp4").is_file());
    assert!(output.join("newer/game.mp4").is_file());
}

#[test]
fn a_saved_recording_triggers_the_cleanup() {
    let settings = limited("limit-saved", 50);
    old_game(&settings.output_dir, "older", 1, 52 * GB);
    let output = settings.output_dir.clone();
    let harness = start_idle(settings, Arc::new(FakeMediaSource::new()));
    let startup = wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Notice { .. }).then_some(())
    });
    assert!(startup.is_some(), "startup cleanup");
    old_game(&output, "second", 3, 52 * GB);
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 1 }).ok();
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.folder.clone()),
        _ => None,
    })
    .expect("recording saved");
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Notice { .. }).then_some(())
    })
    .expect("cleanup after save");
    assert!(!output.join("second/game.mp4").exists());
    assert!(output.join("second").join(META_FILE).is_file());
    assert!(
        saved.join("game.mp4").is_file(),
        "the new recording is kept"
    );
}

#[test]
fn with_the_limit_off_nothing_is_deleted() {
    let settings = Settings {
        output_dir: temp_dir("limit-off"),
        ..Settings::default()
    };
    old_game(&settings.output_dir, "big", 1, 4_000_000);
    let output = settings.output_dir.clone();
    let harness = start_idle(settings, Arc::new(FakeMediaSource::new()));
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Library(_)).then_some(())
    })
    .expect("startup scan");
    assert!(output.join("big/game.mp4").is_file());
}
