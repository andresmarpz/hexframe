use std::sync::atomic::{AtomicU32, Ordering};

use greyscreen_core::game::{GameInfo, GameResult, Player, Team};
use greyscreen_core::library::{ClipEntry, GameRecord, META_FILE, META_VERSION, VideoInfo};
use jiff::Zoned;

use super::*;

fn output_dir(tag: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "greyscreen-storage-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("dir");
    dir
}

fn player() -> Player {
    Player {
        name: "Me".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    }
}

fn started(day: u32) -> Zoned {
    format!("2026-11-{day:02}T12:00:00[UTC]")
        .parse()
        .expect("zoned")
}

fn meta(id: &str, day: u32, clips: &[&str]) -> RecordingMeta {
    RecordingMeta {
        version: META_VERSION,
        id: id.into(),
        started_at: started(day),
        duration_secs: 60.0,
        video: VIDEO_FILE.into(),
        game: GameRecord {
            info: GameInfo {
                mode: "CLASSIC".into(),
                map: Some(11),
                me: player(),
                players: vec![player()],
            },
            result: GameResult::Win,
        },
        events: Vec::new(),
        clips: clips
            .iter()
            .map(|file| ClipEntry {
                file: format!("clips/{file}"),
                start_secs: 0.0,
                end_secs: 10.0,
                label: "Kill".into(),
            })
            .collect(),
        encoder: "test".into(),
        video_format: VideoInfo {
            width: 1920,
            height: 1080,
            fps: 60,
        },
        review: Default::default(),
        health: None,
        recovered: false,
    }
}

fn game(output: &Path, id: &str, day: u32, video: usize, clip: usize) -> PathBuf {
    let folder = output.join(id);
    fs::create_dir_all(folder.join("clips")).expect("clips");
    core_library::write_meta(&folder, &meta(id, day, &["a.mp4"])).expect("meta");
    if video > 0 {
        fs::write(folder.join(VIDEO_FILE), vec![0u8; video]).expect("video");
    }
    fs::write(folder.join("clips/a.mp4"), vec![0u8; clip]).expect("clip");
    folder
}

fn none_active() -> ActiveFolder {
    ActiveFolder::default()
}

fn active(id: &str) -> ActiveFolder {
    Arc::new(Mutex::new(Some(id.to_owned())))
}

fn total(output: &Path) -> u64 {
    folder_bytes(output)
}

#[test]
fn under_the_limit_nothing_changes() {
    let out = output_dir("under");
    game(&out, "a", 1, 10_000, 1_000);
    let before = total(&out);
    let cleanup = enforce(&out, before, &none_active());
    assert_eq!(cleanup, Cleanup::default());
    assert_eq!(total(&out), before);
}

#[test]
fn the_oldest_game_video_goes_and_the_game_stays() {
    let out = output_dir("oldest");
    let old = game(&out, "old", 1, 10_000, 1_000);
    let new = game(&out, "new", 2, 10_000, 1_000);
    let limit = total(&out) - 100;
    let cleanup = enforce(&out, limit, &none_active());
    assert_eq!(
        cleanup,
        Cleanup {
            videos: 1,
            games: 0
        }
    );
    assert!(!old.join(VIDEO_FILE).exists());
    assert!(old.join(META_FILE).is_file());
    assert!(old.join("clips/a.mp4").is_file());
    assert!(new.join(VIDEO_FILE).is_file());
    assert!(total(&out) <= limit);
}

#[test]
fn a_games_video_field_is_cleared_when_its_video_goes() {
    let out = output_dir("meta");
    game(&out, "old", 1, 10_000, 1_000);
    game(&out, "new", 2, 10_000, 1_000);
    enforce(&out, total(&out) - 100, &none_active());
    let old = core_library::read_meta(&out.join("old")).expect("meta");
    assert_eq!(old.video, "");
    assert_eq!(old.clips.len(), 1);
    let new = core_library::read_meta(&out.join("new")).expect("meta");
    assert_eq!(new.video, VIDEO_FILE);
}

#[test]
fn videos_go_oldest_first_until_the_total_fits() {
    let out = output_dir("many");
    for (id, day) in [("c", 3), ("a", 1), ("b", 2)] {
        game(&out, id, day, 10_000, 100);
    }
    let limit = 16_000;
    let cleanup = enforce(&out, limit, &none_active());
    assert_eq!(cleanup.videos, 2);
    assert!(!out.join("a").join(VIDEO_FILE).exists());
    assert!(!out.join("b").join(VIDEO_FILE).exists());
    assert!(out.join("c").join(VIDEO_FILE).is_file());
}

#[test]
fn whole_games_go_only_when_videos_are_not_enough() {
    let out = output_dir("whole");
    game(&out, "old", 1, 1_000, 50_000);
    game(&out, "new", 2, 1_000, 50_000);
    let cleanup = enforce(&out, 60_000, &none_active());
    assert_eq!(cleanup.games, 1);
    assert!(!out.join("old").exists());
    assert!(out.join("new").join(META_FILE).is_file());
    assert!(out.join("new").join("clips/a.mp4").is_file());
}

#[test]
fn a_game_with_its_video_gone_can_still_be_removed_whole() {
    let out = output_dir("clips-only");
    game(&out, "old", 1, 0, 50_000);
    game(&out, "new", 2, 5_000, 50_000);
    let cleanup = enforce(&out, 60_000, &none_active());
    assert_eq!(
        cleanup,
        Cleanup {
            videos: 1,
            games: 1
        }
    );
    assert!(!out.join("old").exists());
    assert!(out.join("new").join("clips/a.mp4").is_file());
}

#[test]
fn the_recording_in_progress_is_never_touched() {
    let out = output_dir("live");
    let live = game(&out, "live", 1, 10_000, 1_000);
    let other = game(&out, "other", 2, 10_000, 1_000);
    let cleanup = enforce(&out, 0, &active("live"));
    assert_eq!(cleanup.games, 1);
    assert!(live.join(VIDEO_FILE).is_file());
    assert!(live.join("clips/a.mp4").is_file());
    assert!(!other.exists());
}

#[test]
fn the_active_game_counts_toward_the_total() {
    let out = output_dir("counts");
    let live = game(&out, "live", 2, 10_000, 100);
    let old = game(&out, "old", 1, 10_000, 100);
    let limit = total(&out) - 50;
    enforce(&out, limit, &active("live"));
    assert!(live.join(VIDEO_FILE).is_file());
    assert!(!old.join(VIDEO_FILE).exists());
}

#[test]
fn folders_without_game_json_are_ignored() {
    let out = output_dir("foreign");
    let stray = out.join("holiday-photos");
    fs::create_dir_all(&stray).expect("stray");
    fs::write(stray.join(VIDEO_FILE), vec![0u8; 100_000]).expect("file");
    fs::write(out.join("notes.txt"), "keep").expect("file");
    let old = game(&out, "old", 1, 1_000, 100);
    enforce(&out, 0, &none_active());
    assert!(stray.join(VIDEO_FILE).is_file());
    assert!(out.join("notes.txt").is_file());
    assert!(!old.exists());
}

#[test]
fn a_folder_with_a_broken_game_json_is_ignored() {
    let out = output_dir("broken");
    let broken = out.join("broken");
    fs::create_dir_all(&broken).expect("dir");
    fs::write(broken.join(META_FILE), "{ nope").expect("meta");
    fs::write(broken.join(VIDEO_FILE), vec![0u8; 5_000]).expect("video");
    enforce(&out, 0, &none_active());
    assert!(broken.join(VIDEO_FILE).is_file());
}

#[cfg(unix)]
#[test]
fn nothing_outside_the_output_folder_is_touched() {
    let elsewhere = output_dir("elsewhere");
    let target = game(&elsewhere, "real", 1, 10_000, 100);
    let out = output_dir("link");
    std::os::unix::fs::symlink(&target, out.join("real")).expect("symlink");
    let cleanup = enforce(&out, 0, &none_active());
    assert_eq!(cleanup, Cleanup::default());
    assert!(target.join(VIDEO_FILE).is_file());
    assert!(target.join(META_FILE).is_file());
}

#[test]
fn a_missing_output_folder_is_a_no_op() {
    let out = output_dir("missing").join("gone");
    assert_eq!(enforce(&out, 0, &none_active()), Cleanup::default());
}

#[test]
fn running_reports_a_notice_and_the_new_library() {
    let out = output_dir("run");
    game(&out, "old", 1, 10_000, 1_000);
    game(&out, "new", 2, 10_000, 1_000);
    let seen: Arc<Mutex<Vec<UiMsg>>> = Arc::default();
    let sink: Sink = {
        let seen = seen.clone();
        Arc::new(move |msg| seen.lock().expect("lock").push(msg))
    };
    run(&out, Some(total(&out) - 100), &none_active(), &sink);
    let seen = seen.lock().expect("lock");
    assert!(matches!(
        &seen[0],
        UiMsg::Notice { level: NoticeLevel::Info, text }
            if text == "Storage limit reached: deleted 1 old game video."
    ));
    let UiMsg::Library(entries) = &seen[1] else {
        panic!("expected the library, got {:?}", seen[1]);
    };
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .any(|e| e.meta.id == "old" && e.size_bytes == 1_000)
    );
}

#[test]
fn running_without_a_limit_or_a_cleanup_says_nothing() {
    let out = output_dir("quiet");
    game(&out, "a", 1, 10_000, 1_000);
    let seen: Arc<Mutex<Vec<UiMsg>>> = Arc::default();
    let sink: Sink = {
        let seen = seen.clone();
        Arc::new(move |msg| seen.lock().expect("lock").push(msg))
    };
    run(&out, None, &none_active(), &sink);
    run(&out, Some(u64::MAX), &none_active(), &sink);
    assert!(seen.lock().expect("lock").is_empty());
    assert!(out.join("a").join(VIDEO_FILE).is_file());
}

#[test]
fn notices_read_naturally() {
    let text = |videos, games| Cleanup { videos, games }.notice();
    assert_eq!(
        text(3, 0),
        "Storage limit reached: deleted 3 old game videos."
    );
    assert_eq!(text(0, 1), "Storage limit reached: deleted 1 whole game.");
    assert_eq!(
        text(2, 2),
        "Storage limit reached: deleted 2 old game videos and 2 whole games."
    );
}

#[test]
fn freeing_space_deletes_only_old_videos_until_enough() {
    let out = output_dir("free-up");
    let oldest = game(&out, "oldest", 1, 10_000, 1_000);
    let middle = game(&out, "middle", 2, 10_000, 1_000);
    let newest = game(&out, "newest", 3, 10_000, 1_000);

    let cleanup = free_up(&out, 15_000, &none_active());

    assert_eq!(
        cleanup,
        Cleanup {
            videos: 2,
            games: 0
        }
    );
    for folder in [&oldest, &middle] {
        assert!(!folder.join(VIDEO_FILE).exists());
        assert!(folder.join(META_FILE).is_file());
        assert!(folder.join("clips/a.mp4").is_file());
    }
    assert!(newest.join(VIDEO_FILE).is_file());
}

#[test]
fn freeing_space_never_touches_the_game_being_recorded() {
    let out = output_dir("free-up-active");
    let running = game(&out, "running", 1, 10_000, 1_000);
    let older = game(&out, "older", 2, 10_000, 1_000);

    let cleanup = free_up(&out, u64::MAX, &active("running"));

    assert_eq!(cleanup.videos, 1);
    assert!(running.join(VIDEO_FILE).is_file());
    assert!(!older.join(VIDEO_FILE).exists());
}

#[test]
fn freeing_space_deletes_no_whole_game_even_when_it_cannot_reach_the_target() {
    let out = output_dir("free-up-short");
    let only = game(&out, "only", 1, 100, 1_000);

    let cleanup = free_up(&out, u64::MAX, &none_active());

    assert_eq!(
        cleanup,
        Cleanup {
            videos: 1,
            games: 0
        }
    );
    assert!(only.join(META_FILE).is_file());
    assert!(only.join("clips/a.mp4").is_file());
}

#[test]
fn the_low_disk_notice_counts_the_videos() {
    let one = Cleanup {
        videos: 1,
        games: 0,
    };
    let three = Cleanup {
        videos: 3,
        games: 0,
    };

    assert_eq!(
        one.low_disk_notice(),
        "Low disk space: deleted 1 old game video to make room."
    );
    assert_eq!(
        three.low_disk_notice(),
        "Low disk space: deleted 3 old game videos to make room."
    );
}
