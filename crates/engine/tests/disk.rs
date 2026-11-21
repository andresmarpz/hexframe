#![allow(clippy::expect_used)]

mod common;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use greyscreen_core::game::{GameInfo, GameResult, Player, Team};
use greyscreen_core::library::{self, GameRecord, META_FILE, META_VERSION, RecordingMeta, VideoInfo};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    EngineCmd, EngineStatus, FakeMediaSource, FreeSpace, LOW_DISK_URL, NoticeAction, NoticeLevel,
    UiMsg,
};

use common::{
    Harness, WAIT, assert_playable_from_key_frame, no_league, start_engine_with_space, temp_dir,
    wait_for,
};

const GB: u64 = 1_000_000_000;
const MB: u64 = 1_000_000;

struct Fixed(AtomicU64);

impl Fixed {
    fn new(bytes: u64) -> Arc<Self> {
        Arc::new(Self(AtomicU64::new(bytes)))
    }

    fn set(&self, bytes: u64) {
        self.0.store(bytes, Ordering::SeqCst);
    }
}

impl FreeSpace for Fixed {
    fn free_bytes(&self, _dir: &Path) -> Option<u64> {
        Some(self.0.load(Ordering::SeqCst))
    }

    fn check_every(&self) -> Duration {
        Duration::from_millis(50)
    }
}

struct Drive {
    capacity: u64,
    ballast: AtomicU64,
}

impl Drive {
    fn new(capacity: u64) -> Arc<Self> {
        Arc::new(Self {
            capacity,
            ballast: AtomicU64::new(0),
        })
    }
}

fn used(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| Some((entry.path(), entry.metadata().ok()?)))
        .map(|(path, meta)| {
            if meta.is_dir() {
                used(&path)
            } else {
                meta.len()
            }
        })
        .sum()
}

impl FreeSpace for Drive {
    fn free_bytes(&self, dir: &Path) -> Option<u64> {
        let taken = used(dir) + self.ballast.load(Ordering::SeqCst);
        Some(self.capacity.saturating_sub(taken))
    }

    fn check_every(&self) -> Duration {
        Duration::from_millis(50)
    }
}

fn old_game(output: &Path, id: &str, day: u32, video_bytes: u64) -> PathBuf {
    let player = Player {
        name: "Me".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    };
    let folder = output.join(id);
    fs::create_dir_all(&folder).expect("folder");
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
        clips: Vec::new(),
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
    sparse_file(&folder.join("game.mp4"), video_bytes).expect("sparse video");
    folder
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

fn settings(tag: &str, limit_gb: Option<u32>) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        storage_limit_gb: limit_gb,
        ..Settings::default()
    }
}

fn start(settings: Settings, space: Arc<dyn FreeSpace>, speed: u32) -> Harness {
    start_engine_with_space(
        settings,
        Arc::new(FakeMediaSource::new().with_speed(speed)),
        no_league(),
        None,
        Some(space),
    )
}

fn test_recording(harness: &Harness, secs: u32) {
    harness
        .engine
        .as_ref()
        .expect("engine")
        .cmd
        .send(EngineCmd::StartTestRecording { secs })
        .ok();
}

fn notice_text(harness: &Harness, wanted: &str) -> (NoticeLevel, String, Option<NoticeAction>) {
    wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice { level, text } if text.contains(wanted) => {
            Some((*level, text.clone(), None))
        }
        UiMsg::NoticeWithAction {
            level,
            text,
            action,
        } if text.contains(wanted) => Some((*level, text.clone(), Some(action.clone()))),
        _ => None,
    })
    .expect("notice")
}

fn recording_folders(output: &Path) -> Vec<PathBuf> {
    fs::read_dir(output)
        .map(|entries| entries.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

#[test]
fn a_start_below_the_threshold_is_refused_with_a_plain_notice() {
    let settings = settings("disk-refused", None);
    let output = settings.output_dir.clone();
    let harness = start(settings, Fixed::new(1_400 * MB), 20);

    test_recording(&harness, 5);

    let (level, text, action) = notice_text(&harness, "Not enough disk space");
    assert_eq!(level, NoticeLevel::Error);
    assert_eq!(
        text,
        "Not enough disk space to record (1.4 GB free). Free up space or change the recordings folder."
    );
    assert_eq!(action, Some(NoticeAction::Link(LOW_DISK_URL.to_owned())));
    let mut last = None;
    while let Ok(msg) = harness.ui.recv_timeout(Duration::from_millis(500)) {
        if let UiMsg::Status(status) = msg {
            last = Some(status);
        }
    }
    assert_eq!(
        last,
        Some(EngineStatus::Error(
            "Not enough disk space to record".to_owned()
        ))
    );
    assert!(recording_folders(&output).is_empty());
}

#[test]
fn a_start_with_enough_room_goes_ahead() {
    let settings = settings("disk-enough", None);
    let harness = start(settings, Fixed::new(2 * GB), 20);

    test_recording(&harness, 1);

    let saved = wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::RecordingSaved(_)).then_some(())
    });
    assert!(saved.is_some());
}

#[test]
fn a_game_start_that_is_refused_says_so_once_and_starts_when_space_comes_back() {
    let settings = Settings {
        output_dir: temp_dir("disk-game-retry"),
        ..Settings::default()
    };
    let space = Fixed::new(GB);
    let (out_tx, out_rx) = crossbeam_channel::unbounded();
    let league: greyscreen_engine::LeagueFactory = Box::new(
        move |out: crossbeam_channel::Sender<greyscreen_league::LeagueMsg>| {
            out_tx.send(out).ok();
            Box::new(())
        },
    );
    let harness = start_engine_with_space(
        settings,
        Arc::new(FakeMediaSource::new().with_speed(20)),
        league,
        None,
        Some(space.clone()),
    );
    let league = out_rx.recv_timeout(WAIT).expect("league sender");
    league
        .send(greyscreen_league::LeagueMsg::WindowFound { hwnd: 7, pid: 42 })
        .ok();
    league
        .send(greyscreen_league::LeagueMsg::Ready(common::info()))
        .ok();
    notice_text(&harness, "Not enough disk space");

    let mut refusals_again = 0;
    while let Ok(msg) = harness.ui.recv_timeout(Duration::from_secs(7)) {
        if matches!(&msg, UiMsg::NoticeWithAction { text, .. } if text.starts_with("Not enough")) {
            refusals_again += 1;
        }
    }
    assert_eq!(refusals_again, 0);
    space.set(50 * GB);

    let recording = wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })).then_some(())
    });
    assert!(recording.is_some());
}

#[test]
fn the_storage_limit_deletes_old_videos_before_a_start_is_refused() {
    let settings = settings("disk-prune", Some(250));
    let output = settings.output_dir.clone();
    let older = old_game(&output, "older", 1, 30 * GB);
    let newer = old_game(&output, "newer", 2, 30 * GB);
    let harness = start(settings, Drive::new(61 * GB), 20);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Library(_)).then_some(())
    });

    test_recording(&harness, 1);

    let (level, text, action) = notice_text(&harness, "Low disk space");
    assert_eq!(level, NoticeLevel::Info);
    assert_eq!(
        text,
        "Low disk space: deleted 1 old game video to make room."
    );
    assert_eq!(action, None);
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.folder.clone()),
        _ => None,
    })
    .expect("recording started and saved after the cleanup");
    assert!(!older.join("game.mp4").exists());
    assert!(older.join(META_FILE).is_file());
    assert!(newer.join("game.mp4").is_file());
    assert!(saved.join("game.mp4").is_file());
}

#[test]
fn without_a_storage_limit_nothing_is_deleted_and_the_start_is_refused() {
    let settings = settings("disk-no-prune", None);
    let output = settings.output_dir.clone();
    let old = old_game(&output, "old", 1, 60 * GB);
    let harness = start(settings, Drive::new(61 * GB), 20);

    test_recording(&harness, 1);

    notice_text(&harness, "Not enough disk space");
    assert!(old.join("game.mp4").is_file());
}

#[test]
fn a_start_is_refused_when_deleting_old_videos_is_not_enough() {
    let settings = settings("disk-prune-short", Some(250));
    let output = settings.output_dir.clone();
    let old = old_game(&output, "old", 1, 500 * MB);
    let harness = start(settings, Drive::new(1_400 * MB), 20);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Library(_)).then_some(())
    });

    test_recording(&harness, 1);

    let (_, text, _) = notice_text(&harness, "Not enough disk space");
    assert!(text.contains("(1.4 GB free)"), "{text}");
    assert!(!old.join("game.mp4").exists());
    assert!(old.join(META_FILE).is_file());
}

#[test]
fn a_recording_stops_cleanly_when_the_disk_fills_up() {
    let settings = settings("disk-mid", None);
    let space = Fixed::new(50 * GB);
    let harness = start(settings, space.clone(), 5);

    test_recording(&harness, 60);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })).then_some(())
    })
    .expect("recording");
    space.set(300 * MB);

    let (level, text, action) = notice_text(&harness, "Recording stopped");
    assert_eq!(level, NoticeLevel::Error);
    assert_eq!(
        text,
        "Recording stopped: the disk is almost full (0.3 GB free). Free up space or change the recordings folder."
    );
    assert_eq!(action, Some(NoticeAction::Link(LOW_DISK_URL.to_owned())));
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording finalized");
    assert_eq!(saved.meta.video, "game.mp4");
    assert!(
        saved.meta.duration_secs < 59.0,
        "{}",
        saved.meta.duration_secs
    );
    assert!(!saved.folder.join("game.mp4.part").exists());
    assert_playable_from_key_frame(&saved.folder.join("game.mp4"));
}

#[test]
fn a_recording_keeps_going_between_the_two_thresholds() {
    let settings = settings("disk-between", None);
    let space = Fixed::new(50 * GB);
    let harness = start(settings, space.clone(), 20);

    test_recording(&harness, 2);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })).then_some(())
    })
    .expect("recording");
    space.set(GB);

    let mut stopped_for_disk = false;
    let mut saved = false;
    while let Ok(msg) = harness.ui.recv_timeout(WAIT) {
        match msg {
            UiMsg::NoticeWithAction { text, .. } if text.starts_with("Recording stopped") => {
                stopped_for_disk = true;
            }
            UiMsg::RecordingSaved(_) => {
                saved = true;
                break;
            }
            _ => {}
        }
    }
    assert!(saved);
    assert!(!stopped_for_disk);
}

#[test]
fn the_storage_limit_makes_room_before_a_running_recording_is_stopped() {
    let settings = settings("disk-mid-prune", Some(250));
    let output = settings.output_dir.clone();
    let old = old_game(&output, "old", 1, 30 * GB);
    let drive = Drive::new(40 * GB);
    let harness = start(settings, drive.clone(), 5);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Library(_)).then_some(())
    });

    test_recording(&harness, 60);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })).then_some(())
    })
    .expect("recording");
    drive.ballast.store(9_800 * MB, Ordering::SeqCst);

    let (_, text, _) = notice_text(&harness, "Low disk space");
    assert_eq!(
        text,
        "Low disk space: deleted 1 old game video to make room."
    );
    assert!(!old.join("game.mp4").exists());
    harness
        .engine
        .as_ref()
        .expect("engine")
        .cmd
        .send(EngineCmd::StopRecording)
        .ok();
    let stopped_for_disk = wait_for(&harness.ui, |msg| match msg {
        UiMsg::NoticeWithAction { text, .. } if text.starts_with("Recording stopped") => Some(true),
        UiMsg::RecordingSaved(_) => Some(false),
        _ => None,
    });
    assert_eq!(stopped_for_disk, Some(false));
}
