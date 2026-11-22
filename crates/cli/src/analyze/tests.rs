use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use greyscreen_core::health::{AudioHealth, Level, RecordingHealth};
use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_mp4::aac::asc_lc;
use greyscreen_mp4::fragmented::FragmentedWriter;

use super::*;

const VIDEO_STEP: i64 = 1500;
const AUDIO_STEP: i64 = 1024;

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("greyscreen-analyze-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        Self { dir }
    }

    fn mp4(&self) -> PathBuf {
        self.dir.join("game.mp4")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn video_packet(tick: i64) -> Packet {
    Packet {
        track: Track::Video,
        dts: tick * VIDEO_STEP,
        pts: tick * VIDEO_STEP,
        duration: VIDEO_STEP as u32,
        key: tick % 60 == 0,
        data: Arc::from(vec![1u8; 1_000 + (tick % 5) as usize]),
    }
}

fn audio_packet(index: i64) -> Packet {
    Packet {
        track: Track::Audio,
        dts: index * AUDIO_STEP,
        pts: index * AUDIO_STEP,
        duration: AUDIO_STEP as u32,
        key: true,
        data: Arc::from(vec![2u8; 40]),
    }
}

fn write_media(path: &Path, video: &[i64], audio: &[i64], finish: bool) {
    let video_format = VideoFormat {
        width: 2560,
        height: 1440,
        fps: 60,
        sps: vec![0x67, 0x42, 0x00, 0x1f],
        pps: vec![0x68, 0xce, 0x3c, 0x80],
    };
    let audio_format = AudioFormat {
        sample_rate: 48_000,
        channels: 2,
        bitrate: 192_000,
        asc: asc_lc(48_000, 2),
    };
    let mut writer =
        FragmentedWriter::create(path, &video_format, Some(&audio_format)).expect("create");
    let mut next_audio = audio.iter().copied().peekable();
    for &tick in video {
        writer.push(&video_packet(tick)).expect("video");
        while next_audio
            .peek()
            .is_some_and(|&i| i * AUDIO_STEP * 90_000 / 48_000 <= tick * VIDEO_STEP)
        {
            let index = next_audio.next().expect("peeked");
            writer.push(&audio_packet(index)).expect("audio");
        }
    }
    if finish {
        writer.finish().expect("finish");
    }
}

fn level_of(report: &Report, area: &str) -> Level {
    report
        .verdicts
        .iter()
        .find(|v| v.area == area)
        .unwrap_or_else(|| panic!("no verdict for {area}"))
        .level
}

fn all_audio_for(ticks: i64) -> Vec<i64> {
    (0..ticks * VIDEO_STEP * 48_000 / 90_000 / AUDIO_STEP).collect()
}

#[test]
fn a_clean_ten_second_recording_is_ok_everywhere() {
    let fixture = Fixture::new("clean");
    let video: Vec<i64> = (0..600).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(600), true);

    let report = analyze_path(&fixture.dir).expect("analyze");
    assert_eq!(report.overall, Level::Ok, "{:#?}", report.verdicts);
    assert_eq!(report.video.frames, 600);
    assert_eq!(report.video.nominal_fps, 60);
    assert_eq!(report.video.average_fps, 60.0);
    assert_eq!(report.video.histogram[0].count, 600);
    assert_eq!(report.video.missing_frames, 0);
    assert_eq!(report.video.gaps.count, 0);
    assert_eq!(report.video.keyframes.count, 10);
    assert_eq!(report.video.keyframes.max_secs, 1.0);
    assert!(report.file.finalized);
    assert_eq!((report.file.width, report.file.height), (2560, 1440));
    let audio = report.audio.expect("audio");
    assert_eq!(audio.gap_count, 0);
    assert_eq!(audio.coverage_pct, 100.0);
    assert!(audio.av_diff_ms.abs() < 25.0, "{}", audio.av_diff_ms);
}

#[test]
fn audio_that_starts_late_is_measured_on_the_video_timeline() {
    let fixture = Fixture::new("lateaudio");
    let video: Vec<i64> = (0..600).collect();
    let audio: Vec<i64> = (141..all_audio_for(600).len() as i64).collect();
    write_media(&fixture.mp4(), &video, &audio, true);

    let report = analyze_path(&fixture.mp4()).expect("analyze");
    let audio = report.audio.expect("audio");
    assert!(audio.av_diff_ms.abs() < 25.0, "{}", audio.av_diff_ms);
    assert_eq!(audio.gap_count, 0);
    assert_eq!(audio.coverage_pct, 100.0);
}

#[test]
fn a_one_second_video_gap_is_found_with_its_timestamp() {
    let fixture = Fixture::new("videogap");
    let video: Vec<i64> = (0..300).chain(360..600).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(600), true);

    let report = analyze_path(&fixture.mp4()).expect("analyze");
    let gaps = &report.video.gaps;
    assert_eq!(gaps.count, 1);
    assert_eq!(gaps.listed[0].missing_frames, 60);
    assert!((gaps.listed[0].at_secs - 5.0).abs() < 0.001);
    assert!((gaps.longest_ms - 1016.7).abs() < 0.1);
    assert_eq!(report.video.missing_frames, 60);
    assert_eq!(level_of(&report, "Video gaps"), Level::Bad);
    assert_eq!(level_of(&report, "Video frames"), Level::Bad);
    assert_eq!(report.overall, Level::Bad);
    assert_eq!(report.video.histogram[5].count, 1);
    let text = render::text(&report);
    assert!(text.contains("OVERALL: BAD"), "{text}");
    assert!(text.contains("at 00:05.000"), "{text}");
}

#[test]
fn a_single_dropped_frame_is_counted_but_is_not_a_gap() {
    let fixture = Fixture::new("onedrop");
    let video: Vec<i64> = (0..300).chain(301..600).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(600), true);

    let report = analyze_path(&fixture.mp4()).expect("analyze");
    assert_eq!(report.video.gaps.count, 0);
    assert_eq!(report.video.missing_frames, 1);
    assert_eq!(report.video.histogram[1].count, 1);
    assert_eq!(level_of(&report, "Video frames"), Level::Ok);
}

#[test]
fn an_audio_gap_is_found_and_graded() {
    let fixture = Fixture::new("audiogap");
    let video: Vec<i64> = (0..600).collect();
    let audio: Vec<i64> = all_audio_for(600)
        .into_iter()
        .filter(|i| !(100..140).contains(i))
        .collect();
    write_media(&fixture.mp4(), &video, &audio, true);

    let report = analyze_path(&fixture.mp4()).expect("analyze");
    let audio = report.audio.as_ref().expect("audio");
    assert_eq!(audio.gap_count, 1);
    assert!(
        (audio.longest_gap_ms - 853.3).abs() < 0.1,
        "{}",
        audio.longest_gap_ms
    );
    assert!((audio.gaps[0].at_secs - 100.0 * 1024.0 / 48_000.0).abs() < 0.05);
    assert_eq!(level_of(&report, "Audio continuity"), Level::Bad);
    assert_eq!(level_of(&report, "Video gaps"), Level::Ok);
    assert!(audio.coverage_pct < 100.0);
}

#[test]
fn an_unfinished_file_is_flagged_and_a_cut_one_is_bad() {
    let fixture = Fixture::new("unfinished");
    let video: Vec<i64> = (0..200).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(200), false);
    let part = fixture.dir.join("game.mp4.part");
    std::fs::rename(fixture.mp4(), &part).expect("rename");

    let report = analyze_path(&fixture.dir).expect("analyze");
    assert!(!report.file.finalized);
    assert_eq!(level_of(&report, "File"), Level::Warn);
    assert_eq!(report.video.frames, 180);

    let len = std::fs::metadata(&part).expect("meta").len();
    let cut = std::fs::OpenOptions::new()
        .write(true)
        .open(&part)
        .expect("open");
    cut.set_len(len - 30).expect("truncate");
    drop(cut);
    let report = analyze_path(&part).expect("analyze");
    assert_eq!(level_of(&report, "File"), Level::Bad);
    assert!(report.file.trailing_bytes.is_some_and(|b| b > 0));
}

#[test]
fn health_and_diagnostics_join_the_verdicts() {
    let fixture = Fixture::new("joined");
    let video: Vec<i64> = (0..300).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(300), true);
    let health = RecordingHealth {
        target_fps: 60,
        frames_encoded: 300,
        duration_secs: 5.0,
        frames_expected: 300,
        audio: Some(AudioHealth {
            drift_ms: 400.0,
            ..AudioHealth::default()
        }),
        ..RecordingHealth::default()
    };
    let diag = diag::summarize(
        r#"{"type":"setup","adapter":"GPU","driver_version":"1.2","device_id":9988,"subsystem_id":4660,"revision":3,"hags":"on","hags_support":"stable"}
{"type":"second","t":1.0,"encoded":60,"dropped":0,"repeated":1,"encode_ms_max":3.0,"step_ms_max":2.0,"queue_writer":2,"queue_media":0,"queue_encoder":1,"av_skew_ms":-5.0,"cpu_pct":8.5,"mem_mb":300}
{"type":"second","t":2.0,"encoded":55,"dropped":5,"repeated":0,"encode_ms_max":9.0,"step_ms_max":4.0,"queue_writer":9,"queue_media":1,"queue_encoder":2,"av_skew_ms":12.0,"cpu_pct":11.5,"mem_mb":320}
not json
"#,
    );
    let media = greyscreen_mp4::inspect::inspect(&fixture.mp4()).expect("inspect");
    let report = assemble(&fixture.mp4(), &media, Some(health), Some(diag));

    assert_eq!(level_of(&report, "Health: Audio"), Level::Bad);
    assert_eq!(level_of(&report, "Diagnostics: drops"), Level::Bad);
    let diagnostics = report.diagnostics.as_ref().expect("diag");
    assert_eq!(diagnostics.seconds, 2);
    assert_eq!(diagnostics.dropped_frames, 5);
    assert_eq!(diagnostics.queue_writer_max, 9);
    assert_eq!(diagnostics.cpu_pct_max, Some(11.5));
    assert_eq!(diagnostics.cpu_pct_avg, Some(10.0));
    assert_eq!(diagnostics.mem_mb_max, Some(320));
    assert_eq!(diagnostics.bad_lines, 1);
    let text = render::text(&report);
    assert!(text.contains("GPU GPU (device 0x2704, subsystem 0x00001234, revision 3, driver 1.2, hags on, scheduler support stable)"), "{text}");
}

#[test]
fn json_output_carries_the_report_and_old_recordings_have_no_health() {
    let fixture = Fixture::new("json");
    let video: Vec<i64> = (0..120).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(120), true);
    let report = analyze_path(&fixture.dir).expect("analyze");
    let json = serde_json::to_value(&report).expect("json");
    assert_eq!(json["overall"], "ok");
    assert_eq!(json["video"]["frames"], 120);
    assert!(json["health"].is_null());
    assert!(json["verdicts"].as_array().is_some_and(|v| v.len() >= 7));
    let text = render::text(&report);
    assert!(
        text.contains("No health data (recorded before 0.1.12-preview.14)"),
        "{text}"
    );
}

#[test]
fn a_folder_without_a_video_is_an_error() {
    let fixture = Fixture::new("empty");
    let error = analyze_path(&fixture.dir).expect_err("no video");
    assert!(error.to_string().contains("no game.mp4"), "{error}");
}

#[test]
fn bitrate_windows_cover_the_whole_recording() {
    let fixture = Fixture::new("bitrate");
    let video: Vec<i64> = (0..1500).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(1500), true);
    let report = analyze_path(&fixture.mp4()).expect("analyze");
    let rate = &report.video.bitrate;
    assert_eq!(rate.windows_kbps.len(), 3);
    assert!((480..=500).contains(&rate.avg_kbps), "{}", rate.avg_kbps);
}

#[test]
fn an_hour_long_file_analyzes_in_well_under_a_second() {
    let fixture = Fixture::new("hour");
    let ticks = 60 * 3600;
    let video: Vec<i64> = (0..ticks).collect();
    write_media(&fixture.mp4(), &video, &all_audio_for(ticks), true);
    let started = Instant::now();
    let report = analyze_path(&fixture.mp4()).expect("analyze");
    let elapsed = started.elapsed();
    assert_eq!(report.video.frames, 216_000);
    assert!(elapsed.as_secs_f64() < 1.0, "took {elapsed:?}");
}

#[test]
fn diagnostics_split_drop_seconds_by_game_focus() {
    let diag = diag::summarize(
        r#"{"type":"second","t":1.0,"dropped":0,"foreground":true}
{"type":"second","t":2.0,"dropped":3,"foreground":false}
{"type":"second","t":3.0,"dropped":0,"foreground":false}
{"type":"second","t":4.0,"dropped":1,"foreground":false}
{"type":"second","t":5.0,"dropped":2,"foreground":true}
{"type":"second","t":6.0,"dropped":4}
"#,
    );
    assert_eq!(diag.seconds, 6);
    assert_eq!(diag.seconds_with_focus_info, 5);
    assert_eq!(diag.seconds_in_background, 3);
    assert_eq!(diag.drop_seconds_in_background, 2);
    assert_eq!(diag.drop_seconds_in_foreground, 1);
}

#[test]
fn diagnostics_without_focus_info_report_no_focus_counts() {
    let diag = diag::summarize("{\"type\":\"second\",\"t\":1.0,\"dropped\":2}\n");
    assert_eq!(diag.seconds_with_focus_info, 0);
    assert_eq!(diag.drop_seconds_in_background, 0);
}

#[test]
fn diagnostics_summarize_gpu_load_and_drops_under_saturation() {
    let diag = diag::summarize(
        r#"{"type":"second","t":1.0,"dropped":0,"gpu_3d_pct":50.0,"gpu_3d_self_pct":2.0,"gpu_encode_pct":10.0}
{"type":"second","t":2.0,"dropped":4,"gpu_3d_pct":99.0,"gpu_3d_self_pct":4.0,"gpu_encode_pct":30.0}
{"type":"second","t":3.0,"dropped":1,"gpu_3d_pct":60.0,"gpu_3d_self_pct":0.0}
{"type":"second","t":4.0,"dropped":2}
"#,
    );
    let gpu = diag.gpu.expect("gpu");
    assert_eq!(gpu.seconds, 3);
    assert_eq!(gpu.busy_3d_avg, (50.0 + 99.0 + 60.0) / 3.0);
    assert_eq!(gpu.busy_3d_peak, 99.0);
    assert_eq!(gpu.seconds_3d_saturated, 1);
    assert_eq!(gpu.own_3d_avg, 2.0);
    assert_eq!(gpu.own_3d_peak, 4.0);
    assert_eq!(gpu.encode_avg, Some(20.0));
    assert_eq!(gpu.encode_peak, Some(30.0));
    assert_eq!(gpu.drop_seconds, 2);
    assert_eq!(gpu.drop_seconds_3d_saturated, 1);
}

#[test]
fn diagnostics_without_gpu_fields_report_no_gpu() {
    let diag = diag::summarize("{\"type\":\"second\",\"t\":1.0,\"dropped\":2}\n");
    assert!(diag.gpu.is_none());
    let json = serde_json::to_value(&diag).expect("json");
    assert!(json.get("gpu").is_none());
}

fn rows_with_a_burst() -> Vec<greyscreen_mp4::moov::SampleRow> {
    (0..120)
        .map(|index| greyscreen_mp4::moov::SampleRow {
            offset: 0,
            size: if (30..90).contains(&index) {
                5_000
            } else {
                1_000
            },
            duration: VIDEO_STEP as u32,
            sync: index % 60 == 0,
        })
        .collect()
}

#[test]
fn the_peak_over_any_one_second_window_is_found() {
    let report = video::analyze(&rows_with_a_burst(), 60, None);
    assert_eq!(report.vbv.peak_kbps, 2_400);
    assert!(
        (report.vbv.at_secs - 0.5).abs() < 1e-9,
        "{}",
        report.vbv.at_secs
    );
}

#[test]
fn a_peak_over_the_cap_plus_ten_percent_is_flagged() {
    let report = video::analyze(&rows_with_a_burst(), 60, None);
    let setup = |peak: u32| diag::summarize(&format!(r#"{{"type":"setup","peak":{peak}}}"#));
    let flagged = verdicts::vbv(&report, Some(&setup(2_000_000))).expect("verdict");
    assert_eq!(flagged.level, Level::Warn);
    let within = verdicts::vbv(&report, Some(&setup(2_300_000))).expect("verdict");
    assert_eq!(within.level, Level::Ok);
    assert!(verdicts::vbv(&report, None).is_none());
}
