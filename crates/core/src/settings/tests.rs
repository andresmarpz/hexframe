use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

fn scratch_dir() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "greyscreen-settings-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn config(quality: Quality, resolution: Resolution, fps: u32, native: u32) -> CaptureConfig {
    let settings = Settings {
        quality,
        resolution,
        fps,
        ..Settings::default()
    };
    settings.capture_config(native)
}

#[test]
fn defaults_match_architecture() {
    let s = Settings::default();
    assert!(s.auto_record && s.record_full_game && s.audio);
    assert_eq!(s.quality, Quality::High);
    assert_eq!(s.resolution, Resolution::P1080);
    assert_eq!(s.fps, 60);
    assert_eq!(s.output_dir, paths::default_output_dir());
    assert!(s.clips.enabled);
    assert_eq!((s.clips.pre_secs, s.clips.post_secs), (15, 5));
    assert!(s.clips.kills && s.clips.deaths && s.clips.multikills);
    assert!(s.clips.objectives && s.clips.aces);
    assert!(!s.clips.assists && !s.clips.structures);
}

#[test]
fn save_then_load_round_trips() {
    let dir = scratch_dir();
    let path = dir.join("nested").join("settings.json");
    let s = Settings {
        quality: Quality::Max,
        resolution: Resolution::P720,
        fps: 30,
        capture_cursor: false,
        output_dir: dir.join("videos"),
        clips: ClipSettings {
            assists: true,
            ..ClipSettings::default()
        },
        ..Settings::default()
    };
    s.save(&path).unwrap();
    assert_eq!(Settings::load(&path), s);
    assert!(!dir.join("nested").join("settings.json.tmp").exists());
}

#[test]
fn missing_file_loads_defaults() {
    let dir = scratch_dir();
    assert_eq!(
        Settings::load(&dir.join("absent.json")),
        Settings::default()
    );
}

#[test]
fn invalid_file_loads_defaults() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    assert_eq!(Settings::load(&path), Settings::default());
}

#[test]
fn unknown_fields_are_ignored() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(
        &path,
        r#"{"fps":30,"from_the_future":true,"clips":{"kills":false,"extra":1}}"#,
    )
    .unwrap();
    let s = Settings::load(&path);
    assert_eq!(s.fps, 30);
    assert!(!s.clips.kills);
    assert!(s.clips.deaths);
}

#[test]
fn missing_fields_take_defaults() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"quality":"Max","clips":{"pre_secs":30}}"#).unwrap();
    let s = Settings::load(&path);
    assert_eq!(s.quality, Quality::Max);
    assert_eq!(s.clips.pre_secs, 30);
    assert_eq!(s.clips.post_secs, 5);
    assert_eq!(s.fps, 60);
    assert!(s.auto_record);
}

#[test]
fn validate_snaps_fps_to_nearest_supported() {
    for (input, expected) in [
        (0, 30),
        (24, 30),
        (30, 30),
        (44, 30),
        (46, 60),
        (60, 60),
        (144, 60),
    ] {
        let mut s = Settings {
            fps: input,
            ..Settings::default()
        };
        s.validate();
        assert_eq!(s.fps, expected, "fps {input}");
    }
}

#[test]
fn validate_clamps_clip_seconds() {
    let mut s = Settings::default();
    s.clips.pre_secs = 0;
    s.clips.post_secs = 999;
    s.validate();
    assert_eq!((s.clips.pre_secs, s.clips.post_secs), (1, 120));

    s.clips.pre_secs = 120;
    s.clips.post_secs = 60;
    s.validate();
    assert_eq!((s.clips.pre_secs, s.clips.post_secs), (120, 60));
}

const PRESETS: [(Quality, f64, u32); 3] = [
    (Quality::Balanced, 8.0, 60),
    (Quality::High, 12.0, 70),
    (Quality::Max, 20.0, 80),
];

fn expected_mbps(base: f64, height: u32, fps: u32) -> f64 {
    let pixels = f64::from(height) * f64::from(height) * 16.0 / 9.0;
    base * (pixels / (1920.0 * 1080.0)).powf(0.75) * (f64::from(fps) / 60.0).powf(0.8)
}

#[test]
fn bitrate_table_for_each_preset_and_resolution() {
    let resolutions = [
        (Resolution::P720, 720),
        (Resolution::P1080, 1080),
        (Resolution::P1440, 1440),
    ];
    for (quality, base, target) in PRESETS {
        for (resolution, height) in resolutions {
            for fps in [30, 60] {
                let c = config(quality, resolution, fps, 1440);
                let mean = expected_mbps(base, height, fps) * 1e6;
                assert!((f64::from(c.video_bitrate) - mean).abs() < 2.0);
                assert!((f64::from(c.video_peak) - mean * 2.0).abs() < 3.0);
                assert_eq!(c.quality, Some(target));
            }
        }
    }
}

#[test]
fn bitrate_anchors() {
    let c = config(Quality::High, Resolution::P1080, 60, 1080);
    assert_eq!(c.video_bitrate, 12_000_000);
    assert_eq!(c.video_peak, 24_000_000);
    assert_eq!(c.max_height, Some(1080));
    assert_eq!(c.gop_frames, 60);
    assert_eq!(c.audio_bitrate, 192_000);
    let c = config(Quality::High, Resolution::P1440, 60, 1440);
    assert!((18_400_000..18_600_000).contains(&c.video_bitrate));
    let c = config(Quality::Balanced, Resolution::P1080, 30, 1080);
    assert!((4_550_000..4_650_000).contains(&c.video_bitrate));
    assert_eq!(c.gop_frames, 30);
}

#[test]
fn resolution_never_exceeds_native() {
    let c = config(Quality::High, Resolution::P1440, 60, 1080);
    assert_eq!(c.max_height, Some(1080));
    assert_eq!(c.video_bitrate, 12_000_000);
}

#[test]
fn native_resolution_has_no_height_cap_and_uses_native_pixels() {
    let c = config(Quality::High, Resolution::Native, 60, 1440);
    assert_eq!(c.max_height, None);
    assert!((18_400_000..18_600_000).contains(&c.video_bitrate));
}

fn estimate(quality: Quality, resolution: Resolution, fps: u32) -> f64 {
    Settings {
        quality,
        resolution,
        fps,
        ..Settings::default()
    }
    .estimated_mb_per_minute()
}

#[test]
fn estimate_is_the_scaled_mean_plus_audio() {
    let mb = estimate(Quality::High, Resolution::P1080, 60);
    let expected = (12e6 * 0.6 + 192_000.0) * 60.0 / 8.0 / 1e6;
    assert!((mb - expected).abs() < 1e-9);
}

#[test]
fn estimate_grows_with_quality_resolution_and_fps() {
    for resolution in [Resolution::P720, Resolution::P1080, Resolution::P1440] {
        let balanced = estimate(Quality::Balanced, resolution, 60);
        let high = estimate(Quality::High, resolution, 60);
        let max = estimate(Quality::Max, resolution, 60);
        assert!(balanced < high && high < max);
        assert!(estimate(Quality::High, resolution, 30) < high);
    }
    for quality in [Quality::Balanced, Quality::High, Quality::Max] {
        assert!(estimate(quality, Resolution::P720, 60) < estimate(quality, Resolution::P1080, 60));
        assert!(
            estimate(quality, Resolution::P1080, 60) < estimate(quality, Resolution::P1440, 60)
        );
    }
}

#[test]
fn native_estimate_assumes_1440p() {
    assert_eq!(
        estimate(Quality::High, Resolution::Native, 60),
        estimate(Quality::High, Resolution::P1440, 60)
    );
}

#[test]
fn audio_off_lowers_the_estimate() {
    let off = Settings {
        audio: false,
        ..Settings::default()
    };
    assert!(off.estimated_mb_per_minute() < Settings::default().estimated_mb_per_minute());
}

#[test]
fn audio_flag_is_passed_through() {
    let s = Settings {
        audio: false,
        ..Settings::default()
    };
    assert!(!s.capture_config(1080).audio);
}

#[test]
fn cursor_is_on_by_default_and_reaches_the_capture_config() {
    let s = Settings::default();
    assert!(s.capture_cursor);
    assert!(s.capture_config(1080).cursor);
    let off = Settings {
        capture_cursor: false,
        ..Settings::default()
    };
    assert!(!off.capture_config(1080).cursor);
}

#[test]
fn old_settings_file_without_cursor_loads_with_cursor_on() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"fps": 30, "audio": false}"#).unwrap();
    let s = Settings::load(&path);
    assert_eq!(s.fps, 30);
    assert!(s.capture_cursor);
}
