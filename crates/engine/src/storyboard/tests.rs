use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(any(target_os = "macos", windows))]
use std::time::Instant;

use greyscreen_core::library::write_meta;

use super::*;
use crate::thumbs::tests::entry;

#[cfg(any(target_os = "macos", windows))]
use crate::thumbs::tests::fixture_clip;

#[cfg(any(target_os = "macos", windows))]
const GAME_SECS: f64 = 6.0;

#[cfg(any(target_os = "macos", windows))]
fn game(dir: &Path, secs: f64) -> PathBuf {
    let folder = dir.join("g");
    std::fs::create_dir_all(&folder).unwrap();
    fixture_clip(&folder.join("game.mp4"), secs);
    let mut rec = entry(&folder, &[]);
    rec.meta.duration_secs = secs;
    write_meta(&folder, &rec.meta).unwrap();
    folder
}

fn never() -> bool {
    false
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn a_finished_game_gets_sheets_and_an_index_once() {
    let dir = tempfile::tempdir().unwrap();
    let folder = game(dir.path(), GAME_SECS);
    assert_eq!(build(&folder, &never), Outcome::Built);
    let board = storyboard::read(&folder).expect("index");
    assert_eq!((board.interval_secs, board.frames), (1, 7));
    let (w, h) = board.sheet_size();
    assert_eq!(
        image::image_dimensions(folder.join(&board.sheets[0])).unwrap(),
        (w, h)
    );
    assert_eq!(build(&folder, &never), Outcome::Skipped);
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn a_stopped_build_leaves_no_files_behind() {
    let dir = tempfile::tempdir().unwrap();
    let folder = game(dir.path(), GAME_SECS);
    assert_eq!(build(&folder, &|| true), Outcome::Stopped);
    assert!(storyboard::files(&folder).is_empty());
    assert_eq!(build(&folder, &never), Outcome::Built);
}

#[test]
fn a_game_without_a_video_or_length_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("g");
    std::fs::create_dir_all(&folder).unwrap();
    let mut rec = entry(&folder, &[]);
    write_meta(&folder, &rec.meta).unwrap();
    assert_eq!(build(&folder, &never), Outcome::Skipped);
    std::fs::write(folder.join("game.mp4"), b"not a video").unwrap();
    rec.meta.duration_secs = 0.0;
    write_meta(&folder, &rec.meta).unwrap();
    assert_eq!(build(&folder, &never), Outcome::Skipped);
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn a_video_that_cannot_decode_fails_and_leaves_no_files() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("g");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("game.mp4"), b"not a video").unwrap();
    let rec = entry(&folder, &[]);
    write_meta(&folder, &rec.meta).unwrap();
    assert!(matches!(build(&folder, &never), Outcome::Failed(_)));
    assert!(storyboard::files(&folder).is_empty());
}

struct Worker {
    queue: Queue,
    seen: Arc<Mutex<Vec<UiMsg>>>,
    #[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
    active: ActiveFolder,
}

fn worker(recording: Option<&str>) -> Worker {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let inner = seen.clone();
    let sink: Sink = Arc::new(move |msg| inner.lock().unwrap().push(msg));
    let active = ActiveFolder::default();
    *active.lock().unwrap() = recording.map(str::to_owned);
    Worker {
        queue: spawn(sink, active.clone()),
        seen,
        active,
    }
}

impl Worker {
    #[cfg(any(target_os = "macos", windows))]
    fn wait_for(&self, folder: &Path) -> bool {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if self
                .seen
                .lock()
                .unwrap()
                .contains(&UiMsg::Storyboard(folder.to_path_buf()))
            {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn told(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn the_worker_builds_a_requested_storyboard_and_tells_the_ui() {
    let dir = tempfile::tempdir().unwrap();
    let folder = game(dir.path(), GAME_SECS);
    let worker = worker(None);
    worker.queue.request(folder.clone());
    assert!(worker.wait_for(&folder), "no storyboard in time");
    assert!(storyboard::read(&folder).is_some());
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn the_worker_waits_while_a_game_records_then_builds() {
    let dir = tempfile::tempdir().unwrap();
    let folder = game(dir.path(), GAME_SECS);
    let worker = worker(Some("2026-11-21_live"));
    worker.queue.request(folder.clone());
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(worker.told(), 0);
    assert!(storyboard::files(&folder).is_empty());
    *worker.active.lock().unwrap() = None;
    assert!(worker.wait_for(&folder), "no storyboard after the game");
}

#[test]
fn a_folder_that_is_gone_is_dropped_without_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker(None);
    worker.queue.request(dir.path().join("deleted"));
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(worker.told(), 0);
    assert!(!dir.path().join("deleted").exists());
}
