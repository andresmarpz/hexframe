use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use greyscreen_core::media::Track;

use super::*;

fn test_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!("greyscreen-w-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn packet() -> Arc<Packet> {
    Arc::new(Packet {
        track: Track::Video,
        dts: 0,
        pts: 0,
        duration: 1500,
        key: true,
        data: Arc::from(vec![0u8; 4]),
    })
}

#[test]
fn full_queue_stops_the_writer_without_blocking() {
    let (tx, rx) = bounded(2);
    let mut writer = GameWriter {
        tx: Some(tx),
        thread: None,
    };
    let started = Instant::now();
    assert_eq!(writer.send(packet()), Sent::Queued);
    assert_eq!(writer.send(packet()), Sent::Queued);
    assert_eq!(writer.send(packet()), Sent::Full);
    assert_eq!(writer.send(packet()), Sent::Stopped);
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(rx.iter().count(), 2);
}

#[test]
fn rename_retries_until_the_target_is_free() {
    let dir = test_dir("rename-retry");
    let part = dir.join(PART_FILE);
    let target = dir.join(VIDEO_FILE);
    fs::write(&part, b"data").unwrap();
    fs::create_dir_all(target.join("blocker")).unwrap();
    let blocker = target.clone();
    let release = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        fs::remove_dir_all(blocker).unwrap();
    });
    assert!(rename_with_retry(&part, &target).is_ok());
    release.join().unwrap();
    assert!(target.is_file());
    fs::remove_dir_all(dir).ok();
}

#[test]
fn rename_gives_up_after_five_attempts() {
    let dir = test_dir("rename-fail");
    let part = dir.join(PART_FILE);
    let target = dir.join(VIDEO_FILE);
    fs::write(&part, b"data").unwrap();
    fs::create_dir_all(target.join("blocker")).unwrap();
    let started = Instant::now();
    assert!(rename_with_retry(&part, &target).is_err());
    assert!(started.elapsed() >= Duration::from_millis(800));
    assert!(part.is_file());
    fs::remove_dir_all(dir).ok();
}
