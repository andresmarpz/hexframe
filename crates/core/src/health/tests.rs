use std::sync::Arc;

use super::*;
use crate::media::{Packet, Track, VideoFormat};

fn format() -> VideoFormat {
    VideoFormat {
        width: 1920,
        height: 1080,
        fps: 60,
        sps: Vec::new(),
        pps: Vec::new(),
    }
}

fn video(tick: i64) -> Packet {
    Packet {
        track: Track::Video,
        dts: tick * 1500,
        pts: tick * 1500,
        duration: 1500,
        key: tick % 60 == 0,
        data: Arc::from(vec![0u8; 1000]),
    }
}

fn audio(index: i64) -> Packet {
    Packet {
        track: Track::Audio,
        dts: index * 1024,
        pts: index * 1024,
        duration: 1024,
        key: true,
        data: Arc::from(vec![0u8; 100]),
    }
}

fn tracker() -> HealthTracker {
    HealthTracker::new(0, &format(), "Test encoder (quality 70)", Some(48_000))
}

#[test]
fn clean_recording_has_no_drops_or_gaps() {
    let mut tracker = tracker();
    for tick in 0..600 {
        tracker.on_packet(&video(tick));
    }
    for index in 0..469 {
        tracker.on_packet(&audio(index));
    }
    let health = tracker.finish(Some(1_000_000));
    assert_eq!(health.frames_encoded, 600);
    assert_eq!(health.frames_dropped, 0);
    assert_eq!(health.dropped_pct, 0.0);
    assert_eq!(health.frames_expected, 600);
    assert_eq!(health.longest_video_gap_ms, 16.7);
    assert_eq!(health.duration_secs, 10.0);
    assert_eq!(health.avg_bitrate_kbps, 800);
    let audio = health.audio.clone().expect("audio");
    assert_eq!(audio.gaps, 0);
    assert!(audio.drift_ms.abs() < 25.0, "{}", audio.drift_ms);
    assert_eq!(health.worst(), Level::Ok);
}

#[test]
fn a_one_second_video_gap_counts_the_missing_frames() {
    let mut tracker = tracker();
    for tick in (0..60).chain(120..180) {
        tracker.on_packet(&video(tick));
    }
    let health = tracker.finish(None);
    assert_eq!(health.frames_dropped, 60);
    assert_eq!(health.frames_expected, 180);
    assert_eq!(health.dropped_pct, 33.33);
    assert_eq!(health.longest_video_gap_ms, 1016.7);
    assert_eq!(health.worst(), Level::Bad);
}

#[test]
fn audio_dts_gap_is_counted_and_measured() {
    let mut tracker = tracker();
    for tick in 0..120 {
        tracker.on_packet(&video(tick));
    }
    for index in (0..20).chain(30..90) {
        tracker.on_packet(&audio(index));
    }
    let audio = tracker.finish(None).audio.expect("audio");
    assert_eq!(audio.gaps, 1);
    assert_eq!(audio.longest_gap_ms, 213.3);
    assert_eq!(audio_level(&audio), Level::Warn);
}

#[test]
fn audio_start_offset_and_drift_follow_the_file_timeline() {
    let mut tracker = tracker();
    for tick in 3..183 {
        tracker.on_packet(&video(tick));
    }
    for index in 0..142 {
        tracker.on_packet(&audio(index));
    }
    let audio = tracker.finish(None).audio.expect("audio");
    assert_eq!(audio.start_offset_ms, -7.3);
    assert!(audio.drift_ms.abs() < 25.0);
}

#[test]
fn stats_feed_counters_timing_and_diag_samples() {
    let mut tracker = tracker();
    for tick in 0..60 {
        tracker.on_packet(&video(tick));
    }
    let mut stats = CaptureStats {
        at: 10_000_000,
        captured: 58,
        repeated: 4,
        window_encode_max_us: 7_400,
        in_flight: 2,
        cpu_pct: Some(12.34),
        mem_mb: Some(310),
        ..CaptureStats::default()
    };
    for us in [2_000, 2_200, 2_400, 9_000] {
        stats.encode.record(us);
    }
    let sample = tracker.on_stats(
        &stats,
        Queues {
            writer: 3,
            media: 1,
        },
    );
    assert_eq!(sample.t, 1.0);
    assert_eq!(sample.encoded, 60);
    assert_eq!(sample.captured, 58);
    assert_eq!(sample.repeated, 4);
    assert_eq!(sample.encode_ms_max, 7.4);
    assert_eq!(
        (
            sample.queue_writer,
            sample.queue_media,
            sample.queue_encoder
        ),
        (3, 1, 2)
    );
    for tick in 60..90 {
        tracker.on_packet(&video(tick));
    }
    stats.at = 20_000_000;
    stats.captured = 88;
    let next = tracker.on_stats(&stats, Queues::default());
    assert_eq!((next.encoded, next.captured, next.repeated), (30, 30, 0));

    let health = tracker.finish(None);
    let timing = health.encode_ms.expect("timing");
    assert_eq!(timing.max, 9.0);
    assert!(timing.avg > 3.0 && timing.avg < 4.0);
    assert!(timing.p95 >= timing.avg);
    assert_eq!(health.frames_captured, 88);
    assert_eq!(health.frames_repeated, 4);
    assert_eq!(health.repeated_pct, 4.44);
    assert_eq!(health.source_fps, 58.7);
}

#[test]
fn histogram_percentiles() {
    let mut histogram = LatencyHistogram::default();
    for _ in 0..95 {
        histogram.record(1_000);
    }
    for _ in 0..5 {
        histogram.record(20_000);
    }
    assert_eq!(histogram.count(), 100);
    assert!(histogram.percentile_ms(0.5) <= 1.5);
    assert!(histogram.percentile_ms(0.99) >= 20.0);
    assert_eq!(histogram.max_ms(), 20.0);
    assert_eq!(LatencyHistogram::default().percentile_ms(0.95), 0.0);
}

#[test]
fn failures_are_counted_and_graded() {
    let mut tracker = tracker();
    for tick in 0..60 {
        tracker.on_packet(&video(tick));
    }
    tracker.on_failure();
    let health = tracker.finish(None);
    assert_eq!(health.capture_errors, 1);
    assert!(
        health
            .findings()
            .iter()
            .any(|f| f.area == Area::Capture && f.level == Level::Bad)
    );
}

#[test]
fn health_round_trips_and_old_sidecars_without_it_read() {
    let mut tracker = tracker();
    for tick in 0..120 {
        tracker.on_packet(&video(tick));
    }
    let health = tracker.finish(Some(5_000_000));
    let json = serde_json::to_string(&health).expect("serialize");
    let back: RecordingHealth = serde_json::from_str(&json).expect("parse");
    assert_eq!(back, health);

    let partial: RecordingHealth =
        serde_json::from_str(r#"{"target_fps":30,"future_field":1}"#).expect("partial");
    assert_eq!(partial.target_fps, 30);
    assert_eq!(partial.frames_encoded, 0);
    assert!(partial.audio.is_none());
}

#[test]
fn summary_line_reads_like_the_popover() {
    let health = RecordingHealth {
        target_fps: 60,
        duration_secs: 100.0,
        frames_encoded: 5_980,
        dropped_pct: 0.3,
        avg_bitrate_kbps: 18_000,
        audio: Some(AudioHealth::default()),
        ..RecordingHealth::default()
    };
    assert_eq!(
        health.summary_line(),
        "59.8 fps · 0.3% dropped · audio OK · 18 Mbps"
    );
}

#[test]
fn thresholds_grade_each_area() {
    assert_eq!(Level::grade(0.4, 0.5, 2.0), Level::Ok);
    assert_eq!(Level::grade(0.5, 0.5, 2.0), Level::Warn);
    assert_eq!(Level::grade(2.0, 0.5, 2.0), Level::Bad);
    let slow = RecordingHealth {
        target_fps: 60,
        frames_encoded: 100,
        duration_secs: 2.0,
        encode_ms: Some(EncodeTiming {
            avg: 20.0,
            p95: 40.0,
            max: 60.0,
        }),
        ..RecordingHealth::default()
    };
    assert!(
        slow.findings()
            .iter()
            .any(|f| f.area == Area::Encoder && f.level == Level::Bad)
    );
}
