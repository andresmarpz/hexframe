use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
fn rename_replaces_an_existing_target_file() {
    let dir = test_dir("rename-over");
    let part = dir.join(PART_FILE);
    let target = dir.join(VIDEO_FILE);
    fs::write(&part, b"new").unwrap();
    fs::write(&target, b"old").unwrap();
    assert!(rename_with_retry(&part, &target).is_ok());
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert!(!part.exists());
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
    failed: Arc<AtomicBool>,
}

impl std::io::Write for BrokenDisk {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.full.swap(false, Ordering::SeqCst) {
            self.failed.store(true, Ordering::SeqCst);
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

fn disk(full: &Arc<AtomicBool>, failed: &Arc<AtomicBool>) -> BrokenDisk {
    BrokenDisk {
        bytes: std::io::Cursor::new(Vec::new()),
        full: Arc::clone(full),
        failed: Arc::clone(failed),
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

fn start_on(
    sink: impl Sink + Send + 'static,
    failures: &Arc<AtomicU32>,
) -> (GameWriter, mpsc::Receiver<Report>, PathBuf) {
    start_on_queue(sink, failures, 20_000)
}

fn start_on_queue(
    sink: impl Sink + Send + 'static,
    failures: &Arc<AtomicU32>,
    queue: usize,
) -> (GameWriter, mpsc::Receiver<Report>, PathBuf) {
    let dir = test_dir("sink");
    fs::write(dir.join(PART_FILE), b"").unwrap();
    let (done_tx, done_rx) = mpsc::channel();
    let failures = Arc::clone(failures);
    let writer = GameWriter::spawn_on(
        sink,
        &dir,
        &video_format(),
        None,
        queue,
        move |_| {
            failures.fetch_add(1, Ordering::SeqCst);
        },
        move |r| {
            done_tx.send(r).ok();
        },
    )
    .unwrap();
    (writer, done_rx, dir)
}

fn start_game(
    full: &Arc<AtomicBool>,
    failed: &Arc<AtomicBool>,
) -> (GameWriter, mpsc::Receiver<Report>, PathBuf) {
    start_on(disk(full, failed), &Arc::new(AtomicU32::new(0)))
}

struct FullAfter {
    bytes: std::io::Cursor<Vec<u8>>,
    left: usize,
}

impl std::io::Write for FullAfter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() > self.left {
            return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
        }
        self.left -= buf.len();
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for FullAfter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(pos)
    }
}

impl Sink for FullAfter {
    fn sync(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn full_after(left: usize) -> FullAfter {
    FullAfter {
        bytes: std::io::Cursor::new(Vec::new()),
        left,
    }
}

fn wait_for_failures(failures: &Arc<AtomicU32>, within: Duration) -> Duration {
    let waited = Instant::now();
    while failures.load(Ordering::SeqCst) == 0 {
        assert!(waited.elapsed() < within, "the writer never reported");
        thread::sleep(Duration::from_millis(5));
    }
    waited.elapsed()
}

#[test]
fn a_write_error_that_clears_still_leaves_a_finished_game() {
    let full = Arc::new(AtomicBool::new(false));
    let failed = Arc::new(AtomicBool::new(false));
    let (mut writer, done_rx, dir) = start_game(&full, &failed);
    assert_eq!(writer.send(key_packet(0)), Sent::Queued);
    let opened = Instant::now();
    while writer.queued() > 0 {
        assert!(
            opened.elapsed() < Duration::from_secs(10),
            "header not written"
        );
        thread::sleep(Duration::from_millis(5));
    }
    full.store(true, Ordering::SeqCst);
    for i in 1..5 {
        assert_eq!(writer.send(key_packet(i * 1500)), Sent::Queued);
    }
    let waited = Instant::now();
    while !failed.load(Ordering::SeqCst) {
        assert!(
            waited.elapsed() < Duration::from_secs(10),
            "no write failed"
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert!(done_rx.recv_timeout(Duration::from_millis(200)).is_err());
    writer.close();
    let report = done_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(report.is_ok());
    assert!(dir.join(VIDEO_FILE).is_file());
    drop(writer);
    fs::remove_dir_all(dir).ok();
}

#[test]
fn healthy_sink_finishes_the_game() {
    let full = Arc::new(AtomicBool::new(false));
    let failed = Arc::new(AtomicBool::new(false));
    let (mut writer, done_rx, dir) = start_game(&full, &failed);
    for i in 0..3 {
        assert_eq!(writer.send(key_packet(i * 1500)), Sent::Queued);
    }
    writer.close();
    assert!(
        done_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .is_ok()
    );
    assert!(dir.join(VIDEO_FILE).is_file());
    drop(writer);
    fs::remove_dir_all(dir).ok();
}

#[test]
fn the_first_failed_write_is_reported_at_once_and_the_part_file_stays() {
    let failures = Arc::new(AtomicU32::new(0));
    let (mut writer, done_rx, dir) = start_on(full_after(1200), &failures);
    for i in 0..10 {
        assert_eq!(writer.send(key_packet(i * 1500)), Sent::Queued);
    }
    let heard = wait_for_failures(&failures, Duration::from_secs(2));
    assert!(heard < Duration::from_secs(2));
    writer.close();
    let report = done_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(report.unwrap_err().contains("no storage space"));
    assert_eq!(failures.load(Ordering::SeqCst), 1);
    assert!(!dir.join(VIDEO_FILE).exists());
    drop(writer);
    fs::remove_dir_all(dir).ok();
}

#[test]
fn a_game_of_good_packets_finishes_with_every_sample() {
    let failures = Arc::new(AtomicU32::new(0));
    let (mut writer, done_rx, dir) = start_on(full_after(usize::MAX), &failures);
    for i in 0..300 {
        assert_eq!(writer.send(key_packet(i * 1500)), Sent::Queued);
    }
    writer.close();
    let info = done_rx
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    assert!((info.duration_secs - 300.0 * 1500.0 / 90_000.0).abs() < 0.01);
    assert_eq!(failures.load(Ordering::SeqCst), 0);
    drop(writer);
    fs::remove_dir_all(dir).ok();
}

#[test]
fn ten_thousand_failing_packets_finish_and_report_one_failure() {
    let failures = Arc::new(AtomicU32::new(0));
    let (mut writer, done_rx, dir) = start_on(full_after(1200), &failures);
    for i in 0..10_000 {
        assert_eq!(writer.send(key_packet(i * 1500)), Sent::Queued);
    }
    writer.close();
    assert!(
        done_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .is_err()
    );
    assert_eq!(failures.load(Ordering::SeqCst), 1);
    drop(writer);
    fs::remove_dir_all(dir).ok();
}

struct ScriptedSink {
    bytes: std::io::Cursor<Vec<u8>>,
    stalled: Arc<AtomicBool>,
    panics: Arc<AtomicBool>,
}

impl std::io::Write for ScriptedSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        assert!(!self.panics.load(Ordering::SeqCst), "the disk panicked");
        while self.stalled.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(5));
        }
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for ScriptedSink {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(pos)
    }
}

impl Sink for ScriptedSink {
    fn sync(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Release(Arc<AtomicBool>);

impl Drop for Release {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

struct Scripted {
    _release: Release,
    writer: GameWriter,
    done: mpsc::Receiver<Report>,
    dir: PathBuf,
    stalled: Arc<AtomicBool>,
    panics: Arc<AtomicBool>,
    failures: Arc<AtomicU32>,
}

fn scripted(queue: usize) -> Scripted {
    let stalled = Arc::new(AtomicBool::new(false));
    let panics = Arc::new(AtomicBool::new(false));
    let failures = Arc::new(AtomicU32::new(0));
    let sink = ScriptedSink {
        bytes: std::io::Cursor::new(Vec::new()),
        stalled: Arc::clone(&stalled),
        panics: Arc::clone(&panics),
    };
    let (writer, done, dir) = start_on_queue(sink, &failures, queue);
    Scripted {
        _release: Release(Arc::clone(&stalled)),
        writer,
        done,
        dir,
        stalled,
        panics,
        failures,
    }
}

fn drop_within_timeout(writer: GameWriter) {
    let (gone_tx, gone_rx) = mpsc::channel();
    thread::spawn(move || {
        drop(writer);
        gone_tx.send(()).ok();
    });
    gone_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the writer thread joins");
}

fn report_once(done: &mpsc::Receiver<Report>) -> Report {
    let report = done.recv_timeout(Duration::from_secs(1)).expect("a report");
    assert!(done.try_recv().is_err(), "a second report arrived");
    report
}

#[test]
fn a_stalled_disk_never_blocks_the_sender_and_the_queued_prefix_is_finished() {
    let mut game = scripted(8);
    game.stalled.store(true, Ordering::SeqCst);
    let started = Instant::now();
    let sent: Vec<Sent> = (0..200)
        .map(|i| game.writer.send(key_packet(i * 1500)))
        .collect();
    assert!(started.elapsed() < Duration::from_secs(1));
    let queued = sent.iter().filter(|s| **s == Sent::Queued).count();
    assert!((8..=10).contains(&queued), "{queued}");
    assert_eq!(sent.iter().filter(|s| **s == Sent::Full).count(), 1);
    assert!(sent[queued + 1..].iter().all(|s| *s == Sent::Stopped));
    game.stalled.store(false, Ordering::SeqCst);
    let info = report_once(&game.done).expect("finished prefix");
    assert!(info.duration_secs > 0.0);
    assert!(game.dir.join(VIDEO_FILE).is_file());
    assert_eq!(game.failures.load(Ordering::SeqCst), 0);
    drop_within_timeout(game.writer);
    fs::remove_dir_all(game.dir).ok();
}

#[test]
fn dropping_after_close_joins_and_reports_once() {
    let mut game = scripted(64);
    for i in 0..3 {
        game.writer.send(key_packet(i * 1500));
    }
    game.writer.close();
    drop_within_timeout(game.writer);
    assert!(report_once(&game.done).is_ok());
    fs::remove_dir_all(game.dir).ok();
}

#[test]
fn dropping_an_idle_writer_joins_and_reports_once() {
    let game = scripted(64);
    drop_within_timeout(game.writer);
    assert!(report_once(&game.done).is_err());
    fs::remove_dir_all(game.dir).ok();
}

#[test]
fn dropping_with_a_full_queue_joins_and_reports_once() {
    let mut game = scripted(4);
    game.stalled.store(true, Ordering::SeqCst);
    let mut index = 0;
    while game.writer.send(key_packet(index * 1500)) == Sent::Queued {
        index += 1;
    }
    game.stalled.store(false, Ordering::SeqCst);
    drop_within_timeout(game.writer);
    assert!(report_once(&game.done).is_ok());
    fs::remove_dir_all(game.dir).ok();
}

#[test]
fn dropping_in_the_middle_of_a_write_waits_for_it_and_reports_once() {
    let mut game = scripted(64);
    game.stalled.store(true, Ordering::SeqCst);
    for i in 0..3 {
        game.writer.send(key_packet(i * 1500));
    }
    thread::sleep(Duration::from_millis(100));
    let stalled = Arc::clone(&game.stalled);
    let release = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        stalled.store(false, Ordering::SeqCst);
    });
    drop_within_timeout(game.writer);
    release.join().unwrap();
    assert!(report_once(&game.done).is_ok());
    fs::remove_dir_all(game.dir).ok();
}

#[test]
fn a_disk_that_panics_still_reports_once_and_drop_joins() {
    let mut game = scripted(64);
    game.panics.store(true, Ordering::SeqCst);
    for i in 0..3 {
        game.writer.send(key_packet(i * 1500));
    }
    drop_within_timeout(game.writer);
    let report = report_once(&game.done);
    assert!(report.unwrap_err().contains("panicked"));
    fs::remove_dir_all(game.dir).ok();
}

#[test]
fn a_header_that_cannot_be_written_reports_the_failure_and_one_report() {
    let dir = test_dir("failed-header");
    let (done_tx, done_rx) = mpsc::channel();
    let failures = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&failures);
    let mut writer = GameWriter::spawn_on(
        full_after(0),
        &dir,
        &video_format(),
        None,
        8,
        move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
        },
        move |report| {
            done_tx.send(report).ok();
        },
    )
    .unwrap();
    assert!(
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .is_err()
    );
    assert_eq!(failures.load(Ordering::SeqCst), 1);
    assert!(done_rx.recv_timeout(Duration::from_millis(200)).is_err());
    writer.send(key_packet(0));
    drop(writer);
    fs::remove_dir_all(dir).ok();
}
