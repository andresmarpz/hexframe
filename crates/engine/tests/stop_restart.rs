#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Sender, bounded};
use greyscreen_core::media::{
    CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource, Track, VideoFormat,
};
use greyscreen_core::settings::{Quality, Settings};
use greyscreen_engine::{
    EngineCmd, EngineStatus, EngineTiming, FakeMediaSource, RecordingEntry, UiMsg,
};
use greyscreen_mp4::inspect::{Layout, sample_secs};

use common::{
    Harness, inspect_video, no_league, recording_since, send, start_engine_timed, start_idle,
    temp_dir, wait_for, wait_saved, wait_status,
};

const SAMPLE_RATE: u32 = 48_000;

fn settings(tag: &str) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        ..Settings::default()
    }
}

fn start_test(harness: &Harness, secs: u32) {
    send(harness, EngineCmd::StartTestRecording { secs });
}

fn at_rest(status: &EngineStatus) -> bool {
    matches!(status, EngineStatus::Idle)
}

fn stops_slowly_but_keeps_serving(media: FakeMediaSource, tag: &str, slowest: Duration) {
    let harness = start_idle(settings(tag), Arc::new(media));
    start_test(&harness, 60);
    recording_since(&harness.ui);
    let began = Instant::now();
    send(&harness, EngineCmd::StopRecording);
    wait_status(&harness, |s| matches!(s, EngineStatus::Finalizing));
    send(&harness, EngineCmd::RescanLibrary);
    wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::Library(_)).then_some(())
    })
    .expect("library answered while the stop was blocked");
    assert!(began.elapsed() < slowest, "the stop was not blocking");
    wait_status(&harness, at_rest);
    assert!(began.elapsed() >= slowest, "{:?}", began.elapsed());
    wait_saved(&harness);
}

#[test]
fn a_stop_that_blocks_three_seconds_keeps_the_engine_serving() {
    let media = FakeMediaSource::new().blocking_stop(Duration::from_secs(3));
    stops_slowly_but_keeps_serving(media, "stop-slow", Duration::from_secs(3));
}

#[test]
fn a_stop_that_never_returns_ends_at_the_fake_stop_bound() {
    let media = FakeMediaSource::new()
        .blocking_stop(Duration::from_secs(600))
        .with_stop_bound(Duration::from_secs(2));
    stops_slowly_but_keeps_serving(media, "stop-bound", Duration::from_secs(2));
}

#[test]
fn shutdown_returns_within_its_grace_while_a_stop_is_blocked() {
    let media = FakeMediaSource::new().blocking_stop(Duration::from_secs(3));
    let timing = EngineTiming {
        shutdown_grace: Duration::from_millis(500),
        ..EngineTiming::default()
    };
    let mut harness = start_engine_timed(
        settings("stop-shutdown"),
        Arc::new(media),
        no_league(),
        timing,
    );
    start_test(&harness, 60);
    recording_since(&harness.ui);
    send(&harness, EngineCmd::StopRecording);
    wait_status(&harness, |s| matches!(s, EngineStatus::Finalizing));
    let began = Instant::now();
    drop(harness.engine.take());
    let took = began.elapsed();
    assert!(took < Duration::from_millis(2500), "{took:?}");
}

struct Counting {
    inner: FakeMediaSource,
    video: Arc<AtomicUsize>,
    audio: Arc<AtomicUsize>,
}

struct CountingSession {
    inner: Box<dyn MediaSession>,
    forwarder: JoinHandle<()>,
}

impl MediaSession for CountingSession {
    fn request_keyframe(&self) {}

    fn stop(self: Box<Self>) {
        let this = *self;
        this.inner.stop();
        this.forwarder.join().ok();
    }
}

impl MediaSource for Counting {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        let (tx, rx) = bounded(64);
        let inner = self.inner.start(target, cfg, tx)?;
        let (video, audio) = (self.video.clone(), self.audio.clone());
        let forwarder = thread::spawn(move || {
            for event in rx {
                let track = match &event {
                    MediaEvent::Packet(packet) => Some(packet.track),
                    _ => None,
                };
                if out.send(event).is_err() {
                    continue;
                }
                match track {
                    Some(Track::Video) => video.fetch_add(1, Ordering::SeqCst),
                    Some(Track::Audio) => audio.fetch_add(1, Ordering::SeqCst),
                    None => 0,
                };
            }
        });
        Ok(Box::new(CountingSession { inner, forwarder }))
    }
}

struct Counted {
    _harness: Harness,
    entry: RecordingEntry,
    video: usize,
    audio: usize,
}

fn counted_recording(tag: &str, run_for: Duration) -> Counted {
    let video = Arc::new(AtomicUsize::new(0));
    let audio = Arc::new(AtomicUsize::new(0));
    let media = Counting {
        inner: FakeMediaSource::new().with_speed(20),
        video: video.clone(),
        audio: audio.clone(),
    };
    let harness = start_idle(settings(tag), Arc::new(media));
    start_test(&harness, 600);
    recording_since(&harness.ui);
    thread::sleep(run_for);
    send(&harness, EngineCmd::StopRecording);
    let entry = wait_saved(&harness);
    Counted {
        _harness: harness,
        entry,
        video: video.load(Ordering::SeqCst),
        audio: audio.load(Ordering::SeqCst),
    }
}

fn assert_tracks_match(entry: &RecordingEntry) {
    let inspected = inspect_video(entry);
    let audio = sample_secs(&inspected.tables.audio, SAMPLE_RATE);
    let video = sample_secs(&inspected.tables.video, VideoFormat::TIMESCALE);
    assert!(
        (audio - video).abs() < 0.05,
        "audio {audio} s, video {video} s"
    );
}

fn assert_whole_file(counted: &Counted) {
    let entry = &counted.entry;
    let path = entry.folder.join(&entry.meta.video);
    assert!(!entry.folder.join("game.mp4.part").exists());
    let before = std::fs::metadata(&path).expect("file").len();
    thread::sleep(Duration::from_millis(300));
    assert_eq!(std::fs::metadata(&path).expect("file").len(), before);
    let inspected = inspect_video(entry);
    assert!(matches!(inspected.layout, Layout::Finalized));
    assert_eq!(inspected.tables.video.len(), counted.video);
    assert_eq!(inspected.tables.audio.len(), counted.audio);
    assert!(inspected.tables.video.iter().all(|row| row.duration > 0));
    assert!(inspected.tables.video[0].sync);
    assert_tracks_match(entry);
}

#[test]
fn stop_with_packets_queued_keeps_every_packet_and_writes_nothing_after_close() {
    let counted = counted_recording("stop-queued", Duration::from_millis(700));
    assert!(counted.video > 60, "{}", counted.video);
    assert_whole_file(&counted);
}

#[test]
fn stop_right_after_start_keeps_the_last_packets() {
    let counted = counted_recording("stop-early", Duration::ZERO);
    assert!(counted.video >= 1 && counted.audio >= 1);
    assert_whole_file(&counted);
}

#[test]
fn a_second_recording_in_one_process_starts_clean() {
    let media = FakeMediaSource::new().with_speed(20).with_final_stats();
    let harness = start_idle(settings("restart"), Arc::new(media));
    let mut saved = Vec::new();
    for _ in 0..2 {
        start_test(&harness, 600);
        recording_since(&harness.ui);
        thread::sleep(Duration::from_millis(400));
        send(&harness, EngineCmd::StopRecording);
        saved.push(wait_saved(&harness));
    }
    assert_ne!(saved[0].folder, saved[1].folder);
    for entry in &saved {
        let inspected = inspect_video(entry);
        assert!(matches!(inspected.layout, Layout::Finalized));
        assert!(inspected.tables.video[0].sync);
        assert!(!inspected.video.sps.is_empty() && !inspected.video.pps.is_empty());
        assert_tracks_match(entry);
        let written = inspected.tables.video.len() as u64;
        let health = entry.meta.health.as_ref().expect("health");
        assert_eq!(health.capture_errors, 0);
        assert!(
            health.frames_encoded <= written + 2,
            "{} frames counted, {written} written",
            health.frames_encoded
        );
    }
}

fn late_audio_recording(tag: &str) -> (Harness, RecordingEntry) {
    let media = FakeMediaSource::new()
        .with_speed(20)
        .with_audio_delay_secs(3);
    let harness = start_idle(settings(tag), Arc::new(media));
    start_test(&harness, 600);
    recording_since(&harness.ui);
    thread::sleep(Duration::from_millis(1500));
    send(&harness, EngineCmd::StopRecording);
    let entry = wait_saved(&harness);
    (harness, entry)
}

#[test]
fn a_late_audio_source_shows_up_in_the_health_numbers() {
    let (_harness, entry) = late_audio_recording("late-audio-health");
    let audio = entry.meta.health.expect("health").audio.expect("audio");
    assert!((audio.start_offset_ms - 3000.0).abs() < 50.0, "{audio:?}");
}

#[test]
#[ignore = "bug #79: the finished file loses the audio start offset"]
fn an_audio_source_that_starts_late_keeps_audio_and_video_in_step() {
    let (_harness, entry) = late_audio_recording("late-audio-sync");
    assert_tracks_match(&entry);
}

#[test]
fn changing_fps_and_quality_during_a_recording_keeps_the_running_format() {
    let base = settings("format-change");
    let harness = start_idle(base.clone(), Arc::new(FakeMediaSource::new()));
    start_test(&harness, 600);
    recording_since(&harness.ui);
    let changed = Settings {
        fps: 30,
        quality: Quality::Balanced,
        ..base
    };
    send(&harness, EngineCmd::ApplySettings(changed));
    thread::sleep(Duration::from_millis(500));
    send(&harness, EngineCmd::StopRecording);
    let entry = wait_saved(&harness);
    assert_eq!(inspect_video(&entry).video.fps, 60);
    assert_eq!(entry.meta.video_format.fps, 60);
}
