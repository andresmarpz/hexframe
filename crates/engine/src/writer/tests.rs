use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use greyscreen_core::media::{Track, VideoFormat};

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
    assert!(started.elapsed() < Duration::from_secs(5));
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

struct BrokenDisk {
    bytes: std::io::Cursor<Vec<u8>>,
    full: Arc<AtomicBool>,
}

impl std::io::Write for BrokenDisk {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.full.load(Ordering::SeqCst) {
            return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
        }
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for BrokenDisk {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(pos)
    }
}

impl Sink for BrokenDisk {
    fn sync(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn disk(full: &Arc<AtomicBool>) -> BrokenDisk {
    BrokenDisk {
        bytes: std::io::Cursor::new(Vec::new()),
        full: Arc::clone(full),
    }
}

fn key_packet(dts: i64) -> Arc<Packet> {
    Arc::new(Packet {
        track: Track::Video,
        dts,
        pts: dts,
        duration: 1500,
        key: true,
        data: Arc::from(vec![0u8; 64]),
    })
}

fn video_format() -> VideoFormat {
    VideoFormat {
        width: 640,
        height: 360,
        fps: 60,
        sps: vec![0x67, 0x42, 0xc0, 0x1e],
        pps: vec![0x68, 0xce, 0x3c, 0x80],
    }
}

fn run_game(full: &Arc<AtomicBool>, fill_after: usize, packets: i64) -> Report {
    let dir = test_dir("sink");
    fs::write(dir.join(PART_FILE), b"").unwrap();
    let (done_tx, done_rx) = mpsc::channel();
    let mut writer = GameWriter::spawn_on(disk(full), &dir, &video_format(), None, 16, move |r| {
        done_tx.send(r).ok();
    })
    .unwrap();
    for i in 0..packets {
        if i as usize == fill_after {
            full.store(true, Ordering::SeqCst);
        }
        assert_eq!(writer.send(key_packet(i * 1500)), Sent::Queued);
    }
    writer.close();
    let report = done_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(!dir.join(VIDEO_FILE).exists() || report.is_ok());
    drop(writer);
    fs::remove_dir_all(dir).ok();
    report
}

#[test]
fn write_error_mid_game_is_reported_at_the_end_of_the_game() {
    let full = Arc::new(AtomicBool::new(false));
    assert!(run_game(&full, 1, 5).is_err());
}

#[test]
fn healthy_sink_finishes_the_game() {
    let full = Arc::new(AtomicBool::new(false));
    assert!(run_game(&full, usize::MAX, 3).is_ok());
}
