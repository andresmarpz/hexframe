use greyscreen_core::game::GameResult;
use greyscreen_core::library::{GameRecord, META_VERSION, VideoInfo};
use jiff::Zoned;

use super::*;
use crate::control::meta::manual_game;

fn meta(duration_secs: f64) -> RecordingMeta {
    RecordingMeta {
        version: META_VERSION,
        id: "a".into(),
        started_at: Zoned::now(),
        duration_secs,
        video: String::new(),
        game: GameRecord {
            info: manual_game(),
            result: GameResult::Unknown,
        },
        events: Vec::new(),
        clips: Vec::new(),
        encoder: String::new(),
        video_format: VideoInfo {
            width: 1,
            height: 1,
            fps: 1,
        },
        review: Default::default(),
        health: None,
        recovered: false,
    }
}

#[test]
fn a_queued_sidecar_is_written_by_the_worker() {
    let dir = tempfile::tempdir().unwrap();
    let writer = SidecarWriter::spawn();
    writer.queue(dir.path().to_path_buf(), meta(1.0));
    drop(writer);
    assert_eq!(library::read_meta(dir.path()).unwrap().duration_secs, 1.0);
}

#[test]
fn queued_writes_of_one_folder_end_with_the_newest() {
    let dir = tempfile::tempdir().unwrap();
    let writer = SidecarWriter::spawn();
    for secs in 1..=50 {
        writer.queue(dir.path().to_path_buf(), meta(f64::from(secs)));
    }
    drop(writer);
    assert_eq!(library::read_meta(dir.path()).unwrap().duration_secs, 50.0);
}

#[test]
fn a_final_write_is_never_overwritten_by_an_older_queued_write() {
    let dir = tempfile::tempdir().unwrap();
    let writer = SidecarWriter::spawn();
    for secs in 1..=50 {
        writer.queue(dir.path().to_path_buf(), meta(f64::from(secs)));
    }
    writer.write_final(dir.path(), &meta(99.0)).unwrap();
    drop(writer);
    assert_eq!(library::read_meta(dir.path()).unwrap().duration_secs, 99.0);
}

#[test]
fn queueing_never_waits_for_a_blocked_write() {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let (started_tx, started_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let release_rx = Mutex::new(release_rx);
    let writer = SidecarWriter::spawn_with(Arc::new(move |_, _| {
        started_tx.send(()).ok();
        release_rx.lock().unwrap().recv().ok();
    }));
    let folder = PathBuf::from("blocked");
    writer.queue(folder.clone(), meta(1.0));
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let began = Instant::now();
    writer.queue(folder, meta(2.0));
    assert!(began.elapsed() < Duration::from_millis(500));
    release_tx.send(()).unwrap();
    release_tx.send(()).ok();
    drop(writer);
}
