use super::*;
use crate::settings::Settings;

fn adapter(luid: u64, name: &str, vendor_id: u32) -> Adapter {
    Adapter {
        luid,
        name: name.to_owned(),
        vendor_id,
        driver_version: "1.0".to_owned(),
        ..Adapter::default()
    }
}

fn nvidia() -> Adapter {
    adapter(1, "NVIDIA GeForce RTX 3070", VENDOR_NVIDIA)
}

fn intel() -> Adapter {
    adapter(2, "Intel UHD Graphics", VENDOR_INTEL)
}

fn basic_render() -> Adapter {
    adapter(9, "Microsoft Basic Render Driver", VENDOR_MICROSOFT)
}

fn hw(adapter: u64) -> Candidate {
    Candidate {
        adapter,
        kind: EncoderKind::Hardware,
    }
}

fn sw(adapter: u64) -> Candidate {
    Candidate {
        adapter,
        kind: EncoderKind::Software,
    }
}

#[test]
fn a_single_gpu_tries_its_hardware_encoder_then_the_software_one() {
    let gpu = Adapter {
        display: true,
        high_performance: true,
        power_saving: true,
        ..nvidia()
    };
    assert_eq!(plan(&[gpu, basic_render()]), vec![hw(1), sw(1)]);
}

#[test]
fn the_display_adapter_comes_before_other_gpus() {
    let desktop = [
        Adapter {
            power_saving: true,
            ..intel()
        },
        Adapter {
            display: true,
            high_performance: true,
            ..nvidia()
        },
    ];
    assert_eq!(plan(&desktop), vec![hw(1), hw(2), sw(1)]);
}

#[test]
fn a_hybrid_laptop_tries_the_discrete_gpu_first() {
    let laptop = [
        Adapter {
            display: true,
            power_saving: true,
            ..intel()
        },
        Adapter {
            high_performance: true,
            ..nvidia()
        },
    ];
    assert_eq!(plan(&laptop), vec![hw(1), hw(2), sw(2)]);
}

#[test]
fn a_remote_session_without_a_gpu_only_tries_software() {
    let remote = Adapter {
        display: true,
        ..basic_render()
    };
    assert_eq!(plan(&[remote]), vec![sw(9)]);
    assert_eq!(plan(&[basic_render()]), vec![sw(9)]);
    assert_eq!(plan(&[]), Vec::new());
}

fn attempt(kind: EncoderKind, vendor_id: u32, result: Result<&str, &str>) -> Attempt {
    Attempt {
        kind,
        adapter: "GPU".to_owned(),
        vendor_id,
        result: result.map(str::to_owned).map_err(str::to_owned),
    }
}

#[test]
fn a_working_hardware_encoder_is_the_choice() {
    let check = EncoderCheck::from_attempts(
        "k".into(),
        &[attempt(
            EncoderKind::Hardware,
            VENDOR_NVIDIA,
            Ok("NVIDIA MFT"),
        )],
    );
    assert_eq!(check.outcome, CheckOutcome::Hardware);
    assert_eq!(check.settings_line(), "NVIDIA NVENC (hardware)");
    assert!(!check.wants_notice());
    assert!(!check.failed());
}

#[test]
fn a_failed_hardware_encoder_falls_back_to_software_once_noticed() {
    let mut check = EncoderCheck::from_attempts(
        "k".into(),
        &[
            attempt(EncoderKind::Hardware, VENDOR_AMD, Err("E_FAIL")),
            attempt(EncoderKind::Software, VENDOR_AMD, Ok("Microsoft MFT")),
        ],
    );
    assert_eq!(check.outcome, CheckOutcome::Software);
    assert_eq!(
        check.settings_line(),
        "Windows software encoder — uses more CPU"
    );
    assert_eq!(check.detail, "AMD AMF (hardware): E_FAIL");
    assert!(check.wants_notice());
    check.noticed = true;
    assert!(!check.wants_notice());
}

#[test]
fn nothing_working_is_a_failure_with_every_reason() {
    let check = EncoderCheck::from_attempts(
        "k".into(),
        &[
            attempt(EncoderKind::Hardware, VENDOR_INTEL, Err("no MFT")),
            attempt(EncoderKind::Software, VENDOR_INTEL, Err("E_OUTOFMEMORY")),
        ],
    );
    assert!(check.failed());
    assert_eq!(
        check.detail,
        "Intel Quick Sync (hardware): no MFT; Windows software encoder: E_OUTOFMEMORY"
    );
}

#[test]
fn unknown_vendors_are_named_after_the_adapter() {
    assert_eq!(
        label(EncoderKind::Hardware, 0x1234, "Acme GPU"),
        "Acme GPU (hardware)"
    );
}

#[test]
fn software_encoding_is_capped_at_1080p30() {
    let mut settings = Settings {
        fps: 60,
        ..Settings::default()
    };
    let capped = software_config(&settings.capture_config(1440));
    assert_eq!(capped.fps, 30);
    assert_eq!(capped.gop_frames, 30);
    assert_eq!(capped.max_height, Some(1080));
    settings.fps = 30;
    settings.resolution = crate::settings::Resolution::Height(720);
    let small = software_config(&settings.capture_config(1440));
    assert_eq!((small.fps, small.max_height), (30, Some(720)));
}

#[test]
fn the_check_key_changes_with_drivers_settings_and_version() {
    let cfg = Settings::default().capture_config(1080);
    let base = check_key("NVIDIA 1.0", &cfg, "0.1");
    assert_eq!(base, check_key("NVIDIA 1.0", &cfg, "0.1"));
    assert_ne!(base, check_key("NVIDIA 2.0", &cfg, "0.1"));
    assert_ne!(base, check_key("NVIDIA 1.0", &cfg, "0.2"));
    let other = CaptureConfig {
        fps: cfg.fps / 2,
        ..cfg.clone()
    };
    assert_ne!(base, check_key("NVIDIA 1.0", &other, "0.1"));
}

#[test]
fn the_fingerprint_names_every_driver() {
    assert_eq!(
        fingerprint(&[nvidia(), intel()]),
        "NVIDIA GeForce RTX 3070 0x10de 1.0; Intel UHD Graphics 0x8086 1.0"
    );
}

#[test]
fn encoder_and_device_failures_are_recognised() {
    assert!(is_encoder_failure(&stopped_reason("0x8000ffff")));
    assert!(is_encoder_failure(
        "graphics device lost in encoder drain: removed"
    ));
    assert!(!is_encoder_failure("the captured window closed"));
}

#[test]
fn a_check_survives_a_json_round_trip() {
    let check = EncoderCheck::from_attempts(
        "k".into(),
        &[attempt(EncoderKind::Software, VENDOR_INTEL, Ok("MFT"))],
    );
    let json = serde_json::to_string(&check).unwrap();
    assert!(json.contains("\"outcome\":\"software\""));
    assert_eq!(serde_json::from_str::<EncoderCheck>(&json).unwrap(), check);
}
