use super::*;
use crate::keybinds::{Action, Keymap};
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
    assert_eq!(s.resolution, Resolution::Height(1080));
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
        resolution: Resolution::Height(720),
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
        (Resolution::Height(720), 720),
        (Resolution::Height(1080), 1080),
        (Resolution::Height(1440), 1440),
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
    let c = config(Quality::High, Resolution::Height(1080), 60, 1080);
    assert_eq!(c.video_bitrate, 12_000_000);
    assert_eq!(c.video_peak, 24_000_000);
    assert_eq!(c.max_height, Some(1080));
    assert_eq!(c.gop_frames, 60);
    assert_eq!(c.audio_bitrate, 192_000);
    let c = config(Quality::High, Resolution::Height(1440), 60, 1440);
    assert!((18_400_000..18_600_000).contains(&c.video_bitrate));
    let c = config(Quality::Balanced, Resolution::Height(1080), 30, 1080);
    assert!((4_550_000..4_650_000).contains(&c.video_bitrate));
    assert_eq!(c.gop_frames, 30);
}

#[test]
fn resolution_never_exceeds_native() {
    let c = config(Quality::High, Resolution::Height(1440), 60, 1080);
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
    .estimated_mb_per_minute(1440)
}

#[test]
fn estimate_is_the_scaled_mean_plus_audio() {
    let mb = estimate(Quality::High, Resolution::Height(1080), 60);
    let expected = (12e6 * 0.6 + 192_000.0) * 60.0 / 8.0 / 1e6;
    assert!((mb - expected).abs() < 1e-9);
}

#[test]
fn estimate_grows_with_quality_resolution_and_fps() {
    for resolution in [
        Resolution::Height(720),
        Resolution::Height(1080),
        Resolution::Height(1440),
    ] {
        let balanced = estimate(Quality::Balanced, resolution, 60);
        let high = estimate(Quality::High, resolution, 60);
        let max = estimate(Quality::Max, resolution, 60);
        assert!(balanced < high && high < max);
        assert!(estimate(Quality::High, resolution, 30) < high);
    }
    for quality in [Quality::Balanced, Quality::High, Quality::Max] {
        assert!(
            estimate(quality, Resolution::Height(720), 60)
                < estimate(quality, Resolution::Height(1080), 60)
        );
        assert!(
            estimate(quality, Resolution::Height(1080), 60)
                < estimate(quality, Resolution::Height(1440), 60)
        );
    }
}

#[test]
fn native_estimate_at_1440_matches_a_1440_height() {
    assert_eq!(
        estimate(Quality::High, Resolution::Native, 60),
        estimate(Quality::High, Resolution::Height(1440), 60)
    );
}

#[test]
fn native_estimate_follows_the_given_display_height() {
    assert!(estimate_at(Resolution::Native, 2160) > estimate_at(Resolution::Native, 1440));
    assert_eq!(
        estimate_at(Resolution::Native, 1080),
        estimate_at(Resolution::Height(1080), 1080)
    );
}

#[test]
fn a_height_above_the_display_estimates_at_the_display() {
    assert_eq!(
        estimate_at(Resolution::Height(1440), 1080),
        estimate_at(Resolution::Height(1080), 1080)
    );
}

fn estimate_at(resolution: Resolution, native: u32) -> f64 {
    Settings {
        resolution,
        ..Settings::default()
    }
    .estimated_mb_per_minute(native)
}

#[test]
fn validate_keeps_a_saved_height_in_range() {
    for (input, expected) in [(0, 144), (720, 720), (1200, 1200), (99_999, 4320)] {
        let mut s = Settings {
            resolution: Resolution::Height(input),
            ..Settings::default()
        };
        s.validate();
        assert_eq!(s.resolution, Resolution::Height(expected));
    }
    let mut s = Settings {
        resolution: Resolution::Native,
        ..Settings::default()
    };
    s.validate();
    assert_eq!(s.resolution, Resolution::Native);
}

#[test]
fn audio_off_lowers_the_estimate() {
    let off = Settings {
        audio: false,
        ..Settings::default()
    };
    assert!(off.estimated_mb_per_minute(1440) < Settings::default().estimated_mb_per_minute(1440));
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

#[test]
fn new_settings_default_to_f9_and_no_storage_limit() {
    let s = Settings::default();
    assert_eq!(
        s.keymap().global(),
        vec![(Action::SaveClip, "F9".parse().unwrap())]
    );
    assert!(s.keybinds.is_empty());
    assert_eq!(s.storage_limit_gb, None);
    assert_eq!(s.storage_limit_bytes(), None);
}

#[test]
fn an_old_settings_file_gets_the_new_defaults() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"quality":"Max","fps":30}"#).unwrap();
    let s = Settings::load(&path);
    assert_eq!(s.quality, Quality::Max);
    assert!(s.keybinds.is_empty());
    assert_eq!(s.storage_limit_gb, None);
}

#[test]
fn keybinds_and_storage_limit_survive_a_save() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    let s = Settings {
        keybinds: BTreeMap::from([
            ("save_clip".to_owned(), "Ctrl+Shift+K".to_owned()),
            ("mute".to_owned(), "none".to_owned()),
        ]),
        storage_limit_gb: Some(250),
        ..Settings::default()
    };
    s.save(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"Ctrl+Shift+K\""));
    assert!(!text.contains("save_clip_hotkey"));
    assert_eq!(Settings::load(&path), s);
}

#[test]
fn an_old_hotkey_moves_into_the_keymap() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"fps":30,"save_clip_hotkey":"Ctrl+Shift+K"}"#).unwrap();
    let s = Settings::load(&path);
    assert_eq!(s.fps, 30);
    assert_eq!(
        s.keymap().global(),
        vec![(Action::SaveClip, "Ctrl+Shift+K".parse().unwrap())]
    );
    s.save(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains("save_clip_hotkey"), "{text}");
    assert_eq!(Settings::load(&path).keymap(), s.keymap());
}

#[test]
fn an_old_hotkey_set_to_null_stays_off() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"save_clip_hotkey":null}"#).unwrap();
    assert!(Settings::load(&path).keymap().global().is_empty());
}

#[test]
fn a_new_keybind_wins_over_the_old_hotkey() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    fs::write(
        &path,
        r#"{"save_clip_hotkey":"F5","keybinds":{"save_clip":"F6"}}"#,
    )
    .unwrap();
    assert_eq!(
        Settings::load(&path).keymap().global(),
        vec![(Action::SaveClip, "F6".parse().unwrap())]
    );
}

#[test]
fn a_broken_old_hotkey_falls_back_without_losing_other_settings() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    for bad in [r#""Hyper+Q""#, r#""K""#, r#""""#] {
        fs::write(&path, format!(r#"{{"fps":30,"save_clip_hotkey":{bad}}}"#)).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.fps, 30);
        assert_eq!(
            s.keymap().global(),
            vec![(Action::SaveClip, "F9".parse().unwrap())],
            "{bad}"
        );
    }
}

#[test]
fn malformed_keybinds_do_not_lose_other_settings() {
    let dir = scratch_dir();
    let path = dir.join("settings.json");
    for bad in [r#"[1]"#, r#""x""#, r#"{"mute":5}"#, r#"null"#] {
        fs::write(&path, format!(r#"{{"fps":30,"keybinds":{bad}}}"#)).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.fps, 30, "{bad}");
        assert_eq!(s.keymap(), Keymap::defaults(), "{bad}");
    }
}

#[test]
fn validate_snaps_the_storage_limit_to_a_choice() {
    for (input, expected) in [
        (0, 50),
        (60, 50),
        (80, 100),
        (200, 250),
        (400, 500),
        (9000, 500),
    ] {
        let mut s = Settings {
            storage_limit_gb: Some(input),
            ..Settings::default()
        };
        s.validate();
        assert_eq!(s.storage_limit_gb, Some(expected), "{input}");
    }
}

#[test]
fn validate_keeps_the_storage_limit_off() {
    let mut s = Settings::default();
    s.validate();
    assert_eq!(s.storage_limit_gb, None);
}

#[test]
fn storage_limit_bytes_use_decimal_gigabytes() {
    let s = Settings {
        storage_limit_gb: Some(100),
        ..Settings::default()
    };
    assert_eq!(s.storage_limit_bytes(), Some(100_000_000_000));
}
