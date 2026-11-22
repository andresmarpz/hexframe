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

#[test]
fn driver_versions_read_like_the_vendor_panel() {
    let nvidia = (31i64 << 48) | (15i64 << 16) | 5123;
    assert_eq!(
        driver_version_text(nvidia, 0x10DE),
        "31.0.15.5123 (NVIDIA 551.23)"
    );
    let other = (31i64 << 48) | (101i64 << 16) | 4577;
    assert_eq!(driver_version_text(other, 0x1002), "31.0.101.4577");
}

#[test]
fn torn_frames_and_copy_waits_reach_samples_health_and_findings() {
    let mut tracker = tracker();
    let mut stats = CaptureStats {
        at: 10_000_000,
        torn_dropped: 3,
        window_copy_wait_max_us: 2_500,
        ..CaptureStats::default()
    };
    let first = tracker.on_stats(&stats, Queues::default());
    assert_eq!((first.torn, first.copy_wait_ms_max), (3, 2.5));
    stats.at = 20_000_000;
    stats.torn_dropped = 5;
    stats.copy_wait_slow = 2;
    assert_eq!(tracker.on_stats(&stats, Queues::default()).torn, 2);

    let mut health = tracker.finish(None);
    assert_eq!(
        (
            health.frames_torn_dropped,
            health.copy_wait_slow,
            health.copy_wait_timeouts
        ),
        (5, 2, 0)
    );
    let capture = |health: &RecordingHealth| {
        health
            .findings()
            .into_iter()
            .find(|f| f.area == Area::Capture)
            .map(|f| f.level)
    };
    assert_eq!(capture(&health), Some(Level::Ok));
    health.copy_wait_timeouts = 1;
    assert_eq!(capture(&health), Some(Level::Warn));
    assert_eq!(capture(&RecordingHealth::default()), None);
}

#[test]
fn gpu_instance_names_parse() {
    let name = "pid_1234_luid_0x00000000_0x0000D1B2_phys_0_eng_0_engtype_3D";
    let parsed = gpu::parse_instance(name).expect("parsed");
    assert_eq!(parsed.pid, 1234);
    assert_eq!(parsed.luid, 0xD1B2);
    assert_eq!(parsed.engine, "0_eng_0");
    assert_eq!(parsed.kind, "3D");
    let high = "pid_7_luid_0x00000001_0x0000D1B2_phys_0_eng_3_engtype_VideoEncode";
    let parsed = gpu::parse_instance(high).expect("parsed");
    assert_eq!(parsed.luid, 0x1_0000_D1B2);
    assert_eq!(parsed.kind, "VideoEncode");
    assert!(gpu::parse_instance("garbage").is_none());
    assert!(gpu::parse_instance("pid_x_luid_0x0_0x1_phys_0_eng_0_engtype_3D").is_none());
}

#[test]
fn gpu_usage_sums_per_engine_and_filters_by_adapter() {
    let samples = [
        (
            "pid_10_luid_0x00000000_0x0000AAAA_phys_0_eng_0_engtype_3D",
            60.0,
        ),
        (
            "pid_20_luid_0x00000000_0x0000AAAA_phys_0_eng_0_engtype_3D",
            30.0,
        ),
        (
            "pid_20_luid_0x00000000_0x0000AAAA_phys_0_eng_1_engtype_3D",
            10.0,
        ),
        (
            "pid_20_luid_0x00000000_0x0000AAAA_phys_0_eng_2_engtype_VideoEncode",
            12.0,
        ),
        (
            "pid_10_luid_0x00000000_0x0000AAAA_phys_0_eng_2_engtype_VideoEncode",
            8.0,
        ),
        (
            "pid_20_luid_0x00000000_0x0000AAAA_phys_0_eng_3_engtype_Copy",
            70.0,
        ),
        (
            "pid_10_luid_0x00000000_0x0000AAAA_phys_0_eng_3_engtype_Copy",
            50.0,
        ),
        (
            "pid_10_luid_0x00000000_0x0000BBBB_phys_0_eng_0_engtype_3D",
            99.0,
        ),
    ];
    let usage = gpu::summarize(samples, Some(0xAAAA), 20);
    assert_eq!(usage.busy_3d_pct, 90.0);
    assert_eq!(usage.self_3d_pct, 30.0);
    assert_eq!(usage.encode_pct, 20.0);
    assert_eq!(usage.copy_pct, 100.0);
    let all = gpu::summarize(samples, None, 20);
    assert_eq!(all.busy_3d_pct, 99.0);
}

#[test]
fn a_write_failure_marks_the_video_stopped_once() {
    let mut tracker = tracker();
    for tick in 0..120 {
        tracker.on_packet(&video(tick));
    }
    tracker.on_write_failure();
    for tick in 120..240 {
        tracker.on_packet(&video(tick));
    }
    tracker.on_write_failure();
    let health = tracker.finish(None);
    assert_eq!(health.video_stopped_secs, Some(2.0));
    assert!(
        health
            .findings()
            .iter()
            .any(|f| f.area == Area::File && f.level == Level::Bad)
    );
}

#[test]
fn a_twelve_mbit_peak_gives_the_buffer_size_in_each_unit() {
    assert_eq!(buffer_size(12_000_000, BufferUnit::Bytes), 1_500_000);
    assert_eq!(buffer_size(12_000_000, BufferUnit::Bits), 12_000_000);
}

#[test]
fn each_cap_the_encoder_does_not_keep_is_a_mismatch() {
    assert_eq!(
        cap_mismatch("AVEncCommonMaxBitRate", 12_000_000, Some(12_000_000)),
        None
    );
    let max = cap_mismatch("AVEncCommonMaxBitRate", 12_000_000, Some(6_000_000)).expect("max");
    assert_eq!(max.read_back, Some(6_000_000));
    let buffer = cap_mismatch("AVEncCommonBufferSize", 1_500_000, None).expect("buffer");
    assert_eq!(buffer.read_back, None);
    let text = cap_warning("NVENC", &max);
    assert!(text.contains("NVENC") && text.contains("12000000") && text.contains("6000000"));
}

#[test]
fn setup_warnings_reach_the_health_once() {
    let mut tracker = tracker();
    let warnings = vec!["cap ignored".to_owned()];
    tracker.add_warnings(&warnings);
    tracker.add_warnings(&warnings);
    assert_eq!(tracker.finish(None).warnings, warnings);
}

#[test]
fn wake_lateness_p99_uses_fine_buckets() {
    let mut late = WakeLateness::default();
    for _ in 0..98 {
        late.0.record(40);
    }
    late.0.record(250);
    late.0.record(1_200);
    assert_eq!(late.0.count(), 100);
    assert_eq!(late.0.percentile_us(0.5), 50);
    assert_eq!(late.0.percentile_us(0.99), 300);
    assert_eq!(late.0.max_ms(), 1.2);
}

#[test]
fn spin_share_is_spin_over_wall() {
    let mut share = SpinShare::default();
    assert_eq!(share.pct(), 0.0);
    share.add(100, 1_000);
    share.add(100, 1_000);
    assert!((share.pct() - 10.0).abs() < 0.001);
    share.add(500, 100);
    assert!(share.pct() <= 100.0);
}

#[test]
fn diag_sample_reports_pacer_stats() {
    let mut tracker = HealthTracker::new(0, &format(), "x", None);
    let mut stats = CaptureStats::default();
    stats.wake_lateness.0.record(120);
    stats.pacer_spin.add(30, 1_000);
    stats.first_output_ms = Some(80);
    stats.encoder_stalls_early = 1;
    let sample = tracker.on_stats(&stats, Queues::default());
    assert_eq!(sample.wake_late_p99_us, 120);
    assert!((sample.spin_pct - 3.0).abs() < 0.001);
    assert_eq!(sample.first_output_ms, Some(80));
    assert_eq!(sample.encoder_stalls_early, 1);
}

#[test]
fn loss_causes_stay_apart_and_each_has_its_own_base() {
    let mut stalled = tracker();
    for tick in 0..600 {
        stalled.on_packet(&video(tick));
    }
    stalled.on_stats(
        &CaptureStats {
            captured: 5,
            repeated: 590,
            ..CaptureStats::default()
        },
        Queues::default(),
    );
    let health = stalled.finish(None);
    assert_eq!(health.frames_dropped, 0);
    assert_eq!(health.dropped_pct, 0.0);
    assert_eq!(health.frames_repeated, 590);
    assert!(health.repeated_pct > 90.0);

    let mut holes = tracker();
    for tick in (0..540).chain(570..630) {
        holes.on_packet(&video(tick));
    }
    let health = holes.finish(None);
    assert_eq!(health.frames_encoded, 600);
    assert_eq!(health.frames_dropped, 30);
    assert_eq!(health.dropped_pct, 4.76);
    assert_eq!(health.frames_repeated, 0);

    let fresh = tracker().finish(None);
    assert_eq!(fresh.frames_dropped, 0);
    assert_eq!(fresh.frames_repeated, 0);
}

#[test]
fn audio_trims_reach_the_sample_and_the_summary() {
    let mut tracker = tracker();
    tracker.on_packet(&audio(0));
    let stats = CaptureStats {
        audio_trims: 3,
        audio_trimmed_ms: 120,
        audio_trim_longest_ms: 70,
        ..CaptureStats::default()
    };
    let sample = tracker.on_stats(&stats, Queues::default());
    assert_eq!((sample.audio_trims, sample.audio_trimmed_ms), (3, 120));
    let audio = tracker.finish(None).audio.expect("audio");
    assert_eq!(
        (audio.trims, audio.trimmed_ms, audio.longest_trim_ms),
        (3, 120, 70)
    );
}
