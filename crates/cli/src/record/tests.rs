use super::*;

fn args() -> Args {
    Args {
        seconds: 1.0,
        out: PathBuf::from("x.mp4"),
        window: None,
        fps: 60,
        height: 1440,
        audio: true,
        cursor: false,
        preset: Preset::High,
        rc: None,
        quality: None,
        quality_vs_speed: None,
        bitrate_mbps: None,
        peak_mbps: None,
    }
}

#[test]
fn default_is_quality_mode_from_the_preset() {
    let cfg = build_config(&args()).unwrap();
    assert_eq!(cfg.quality, Some(70));
    assert_eq!(cfg.max_height, Some(1440));
    assert!((18_400_000..18_600_000).contains(&cfg.video_bitrate));
}

#[test]
fn vbr_clears_the_quality_target() {
    let cfg = build_config(&Args {
        rc: Some(RateControl::Vbr),
        ..args()
    })
    .unwrap();
    assert_eq!(cfg.quality, None);
}

#[test]
fn quality_flag_overrides_the_preset_and_vbr_rejects_it() {
    let cfg = build_config(&Args {
        quality: Some(55),
        ..args()
    })
    .unwrap();
    assert_eq!(cfg.quality, Some(55));
    let both = Args {
        rc: Some(RateControl::Vbr),
        quality: Some(55),
        ..args()
    };
    assert!(build_config(&both).is_err());
}

#[test]
fn quality_vs_speed_flag_reaches_the_config_and_defaults_to_unset() {
    assert_eq!(build_config(&args()).unwrap().quality_vs_speed, None);
    let cfg = build_config(&Args {
        quality_vs_speed: Some(50),
        ..args()
    })
    .unwrap();
    assert_eq!(cfg.quality_vs_speed, Some(50));
}

#[test]
fn bitrate_and_peak_override_and_stop_rescaling() {
    let cfg = build_config(&Args {
        bitrate_mbps: Some(10.0),
        ..args()
    })
    .unwrap();
    assert_eq!(
        (cfg.video_bitrate, cfg.video_peak),
        (10_000_000, 20_000_000)
    );
    assert_eq!(cfg.bitrate_pixels, 0);
    let cfg = build_config(&Args {
        bitrate_mbps: Some(10.0),
        peak_mbps: Some(14.0),
        ..args()
    })
    .unwrap();
    assert_eq!(cfg.video_peak, 14_000_000);
}

#[test]
fn preset_changes_the_mean_bitrate() {
    let balanced = build_config(&Args {
        preset: Preset::Balanced,
        ..args()
    })
    .unwrap();
    assert_eq!(balanced.quality, Some(60));
    assert!(balanced.video_bitrate < build_config(&args()).unwrap().video_bitrate);
}
