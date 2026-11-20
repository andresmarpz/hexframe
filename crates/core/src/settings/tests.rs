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

#[test]
fn bitrate_at_1080p60_high_is_20_mbps() {
    let c = config(Quality::High, Resolution::P1080, 60, 1080);
    assert_eq!(c.video_bitrate, 20_000_000);
    assert_eq!(c.video_peak, 30_000_000);
    assert_eq!(c.max_height, Some(1080));
    assert_eq!(c.gop_frames, 60);
    assert_eq!(c.audio_bitrate, 192_000);
}

#[test]
fn bitrate_scales_with_pixels_and_fps() {
    let c = config(Quality::High, Resolution::P1440, 60, 1440);
    assert!((i64::from(c.video_bitrate) - 35_555_556).abs() < 2);
    let c = config(Quality::Balanced, Resolution::P720, 30, 1080);
    assert!((i64::from(c.video_bitrate) - 2_666_667).abs() < 2);
    assert_eq!(c.gop_frames, 30);
}

#[test]
fn resolution_never_exceeds_native() {
    let c = config(Quality::High, Resolution::P1440, 60, 1080);
    assert_eq!(c.max_height, Some(1080));
    assert_eq!(c.video_bitrate, 20_000_000);
}

#[test]
fn native_resolution_has_no_height_cap_and_uses_native_pixels() {
    let c = config(Quality::High, Resolution::Native, 60, 1440);
    assert_eq!(c.max_height, None);
    assert!((i64::from(c.video_bitrate) - 35_555_556).abs() < 2);
}

#[test]
fn audio_flag_is_passed_through() {
    let s = Settings {
        audio: false,
        ..Settings::default()
    };
    assert!(!s.capture_config(1080).audio);
}
