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
        reason: Reason::Failed,
    }
}

#[test]
fn the_failure_line_names_the_adapter_once() {
    let known = EncoderCheck::from_attempts(
        "k".into(),
        &[attempt(EncoderKind::Hardware, VENDOR_NVIDIA, Err("E_FAIL"))],
    );
    assert_eq!(
        known.failure_line(),
        Some("NVIDIA NVENC (hardware) on GPU: E_FAIL")
    );
    let unknown = EncoderCheck::from_attempts(
        "k".into(),
        &[attempt(EncoderKind::Hardware, 0x1234, Err("E_FAIL"))],
    );
    assert_eq!(unknown.failure_line(), Some("GPU (hardware): E_FAIL"));
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
    assert_eq!(
        check.failure_line(),
        Some("AMD AMF (hardware) on GPU: E_FAIL")
    );
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
    let taller = CaptureConfig {
        max_height: Some(cfg.max_height.map_or(720, |h| h + 1)),
        ..cfg.clone()
    };
    assert_ne!(base, check_key("NVIDIA 1.0", &taller, "0.1"));
    let better = CaptureConfig {
        quality: Some(cfg.quality.map_or(80, |q| q + 1)),
        ..cfg.clone()
    };
    assert_ne!(base, check_key("NVIDIA 1.0", &better, "0.1"));
}

#[test]
fn the_fingerprint_names_every_driver() {
    assert_eq!(
        fingerprint(&[nvidia(), intel()]),
        "NVIDIA GeForce RTX 3070 0x10de 0x0000 0x00000000 rev 0 1.0; Intel UHD Graphics 0x8086 0x0000 0x00000000 rev 0 1.0"
    );
}

#[test]
fn the_fingerprint_changes_when_only_the_hardware_ids_differ() {
    let base = nvidia();
    let first = fingerprint(std::slice::from_ref(&base));
    for changed in [
        Adapter {
            device_id: 0x2484,
            ..base.clone()
        },
        Adapter {
            subsystem_id: 0x1234_10de,
            ..base.clone()
        },
        Adapter {
            revision: 2,
            ..base.clone()
        },
    ] {
        assert_ne!(first, fingerprint(&[changed]));
    }
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

#[test]
fn probe_line_round_trips_every_result() {
    for line in [
        ProbeLine::Passed("NVENC, 10 frames".into()),
        ProbeLine::Failed("no device: lost".into()),
        ProbeLine::TimedOut("no answer".into()),
    ] {
        assert_eq!(ProbeLine::parse(&line.format()), Some(line));
    }
}

#[test]
fn probe_line_is_one_line_and_found_among_driver_noise() {
    let line = ProbeLine::Failed("first\nsecond".into()).format();
    assert_eq!(line.lines().count(), 1);
    let output = format!("driver banner\n{line}\n");
    assert_eq!(
        ProbeLine::parse(&output),
        Some(ProbeLine::Failed("first second".into()))
    );
}

#[test]
fn probe_line_rejects_empty_and_unknown_output() {
    assert_eq!(ProbeLine::parse(""), None);
    assert_eq!(ProbeLine::parse("result=maybe detail=x"), None);
    assert_eq!(ProbeLine::parse("something else"), None);
}

#[test]
fn probe_request_round_trips_through_an_argument() {
    let request = ProbeRequest {
        luid: u64::MAX - 5,
        kind: EncoderKind::Software,
        cfg: Settings::default().capture_config(2160),
    };
    assert_eq!(ProbeRequest::from_arg(&request.to_arg()), Some(request));
    assert_eq!(ProbeRequest::from_arg("not json"), None);
}

const BUSY_CODE: u32 = 0x8007_000E;

fn nvidia_probe() -> Probe {
    Probe {
        kind: EncoderKind::Hardware,
        adapter: "NVIDIA GeForce RTX 3070".into(),
        vendor_id: VENDOR_NVIDIA,
        driver_version: "1".into(),
        luid: 1,
    }
}

#[test]
fn every_known_failure_gets_its_own_reason() {
    use OpenStep::*;
    let busy = [BUSY_CODE];
    let other = 0x8000_4005;
    let cases = [
        (Activate, VENDOR_NVIDIA, BUSY_CODE, OpenFailure::Busy),
        (D3dManager, VENDOR_NVIDIA, BUSY_CODE, OpenFailure::Busy),
        (
            Start,
            VENDOR_NVIDIA,
            0x887A_0005,
            OpenFailure::DeviceRemoved,
        ),
        (
            D3dManager,
            VENDOR_NVIDIA,
            0x887A_0007,
            OpenFailure::DeviceRemoved,
        ),
        (Start, VENDOR_NVIDIA, other, OpenFailure::StartFailed),
        (InputType, VENDOR_NVIDIA, other, OpenFailure::InputRefused),
        (D3dManager, VENDOR_NVIDIA, other, OpenFailure::Unsupported),
        (Activate, VENDOR_NVIDIA, other, OpenFailure::Other),
        (OutputType, VENDOR_NVIDIA, other, OpenFailure::Other),
        (Setup, VENDOR_NVIDIA, 0x1234_5678, OpenFailure::Other),
        (Activate, VENDOR_INTEL, BUSY_CODE, OpenFailure::Other),
        (Start, VENDOR_AMD, BUSY_CODE, OpenFailure::StartFailed),
    ];
    for (step, vendor, code, expected) in cases {
        assert_eq!(
            OpenFailure::at_step_with(step, vendor, code, &busy),
            expected,
            "{step:?} {vendor:#x} {code:#x}"
        );
    }
}

#[test]
fn only_a_code_the_measurement_saw_means_busy() {
    assert_eq!(
        OpenFailure::at_step_with(OpenStep::Activate, VENDOR_NVIDIA, BUSY_CODE, &[]),
        OpenFailure::Other
    );
    for code in BUSY_CODES {
        assert_eq!(
            OpenFailure::at_step(OpenStep::Activate, VENDOR_NVIDIA, *code),
            OpenFailure::Busy
        );
    }
}

#[test]
fn a_failure_survives_the_trip_through_error_text() {
    for failure in OpenFailure::ALL {
        let text = failure.tag("SetOutputType failed: oops (0x8007000E)");
        assert_eq!(OpenFailure::from_text(&text), failure);
    }
    assert_eq!(OpenFailure::from_text("plain error"), OpenFailure::Other);
}

#[test]
fn busy_wins_over_other_candidates_in_a_joined_error() {
    let joined = format!(
        "{}; {}",
        OpenFailure::StartFailed.tag("first"),
        OpenFailure::Busy.tag("second")
    );
    assert_eq!(OpenFailure::from_text(&joined), OpenFailure::Busy);
    assert_eq!(Reason::of_error(&joined), Reason::Busy);
    assert_eq!(Reason::of_error("plain"), Reason::Failed);
}

#[test]
fn the_busy_text_appears_only_for_busy_on_nvidia() {
    for failure in OpenFailure::ALL {
        let expected = failure == OpenFailure::Busy;
        assert_eq!(
            failure.help(VENDOR_NVIDIA) == BUSY_HELP,
            expected,
            "{failure:?}"
        );
    }
    assert_eq!(OpenFailure::Busy.help(VENDOR_AMD), FAILED_HELP);
    assert_eq!(OpenFailure::Other.help(VENDOR_NVIDIA), FAILED_HELP);
}

#[test]
fn a_busy_nvidia_verdict_changes_the_notice_and_the_help() {
    let mut check = EncoderCheck::from_attempts(String::new(), &[]);
    assert_eq!(check.notice_text(), SOFTWARE_NOTICE);
    assert_eq!(check.failed_help(), FAILED_HELP);
    check.verdicts.push(Verdict {
        probe: nvidia_probe(),
        reason: Some(Reason::Busy),
    });
    assert_eq!(check.notice_text(), BUSY_HELP);
    assert_eq!(check.failed_help(), BUSY_HELP);
}

#[test]
fn a_busy_software_outcome_is_not_reusable() {
    let mut check = EncoderCheck::from_attempts("k".into(), &[]);
    check.outcome = CheckOutcome::Software;
    check.verdicts.push(Verdict {
        probe: nvidia_probe(),
        reason: Some(Reason::Busy),
    });
    assert!(!check.reusable_for("k"));
}

#[test]
fn a_software_start_after_a_hardware_failure_warns() {
    let busy = [(OpenFailure::Busy, VENDOR_NVIDIA)];
    let other = [(OpenFailure::Other, VENDOR_NVIDIA)];
    assert_eq!(
        fallback_warning(EncoderKind::Software, &busy).as_deref(),
        Some(BUSY_HELP)
    );
    assert_eq!(
        fallback_warning(EncoderKind::Software, &other).as_deref(),
        Some(SOFTWARE_NOTICE)
    );
    assert_eq!(fallback_warning(EncoderKind::Software, &[]), None);
    assert_eq!(fallback_warning(EncoderKind::Hardware, &busy), None);
}

mod planning {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    const VENDORS: [u32; 5] = [
        VENDOR_NVIDIA,
        VENDOR_AMD,
        VENDOR_INTEL,
        VENDOR_MICROSOFT,
        0xbeef,
    ];
    const NAMES: [&str; 3] = ["Intel(R) Arc(TM) A370M", "Intel UHD Graphics", "Unknown"];

    fn adapters() -> impl Strategy<Value = Vec<Adapter>> {
        let one = (
            0..VENDORS.len(),
            0..NAMES.len(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        );
        proptest::collection::btree_map(1u64..40, one, 1..7).prop_map(|map| {
            map.into_iter()
                .map(|(luid, (vendor, name, display, high, saving))| Adapter {
                    luid,
                    name: NAMES[name].to_owned(),
                    vendor_id: VENDORS[vendor],
                    display,
                    high_performance: high,
                    power_saving: saving,
                    ..Adapter::default()
                })
                .collect()
        })
    }

    fn hardware_luids(candidates: &[Candidate]) -> Vec<u64> {
        candidates
            .iter()
            .filter(|c| c.kind == EncoderKind::Hardware)
            .map(|c| c.adapter)
            .collect()
    }

    proptest! {
        #[test]
        fn the_plan_ends_with_one_software_candidate_and_tries_each_gpu_once(list in adapters()) {
            let candidates = plan(&list);
            let software: Vec<usize> = candidates
                .iter()
                .enumerate()
                .filter(|(_, c)| c.kind == EncoderKind::Software)
                .map(|(i, _)| i)
                .collect();
            prop_assert_eq!(software, vec![candidates.len() - 1]);
            let hardware = hardware_luids(&candidates);
            let unique: BTreeSet<u64> = hardware.iter().copied().collect();
            prop_assert_eq!(unique.len(), hardware.len());
            let expected: BTreeSet<u64> =
                list.iter().filter(|a| !a.is_software()).map(|a| a.luid).collect();
            prop_assert_eq!(unique, expected);
        }

        #[test]
        fn the_game_gpu_comes_first_on_a_hybrid_laptop(list in adapters()) {
            let mut sorted = list.clone();
            sorted.sort_by_key(|a| a.luid);
            let hardware: Vec<&Adapter> = sorted.iter().filter(|a| !a.is_software()).collect();
            let display = hardware.iter().find(|a| a.display);
            if let Some(display) = display.filter(|d| d.power_saving && !d.high_performance)
                && hardware.iter().any(|a| a.high_performance && a.luid != display.luid)
            {
                let first = plan(&list)[0];
                let chosen = hardware.iter().find(|a| a.luid == first.adapter).unwrap();
                prop_assert!(chosen.high_performance && chosen.luid != display.luid);
            }
        }

        #[test]
        fn the_plan_ignores_names_and_list_order(list in adapters(), turn in 0usize..7, flip in any::<bool>()) {
            let mut other = list.clone();
            for a in &mut other {
                a.name = format!("renamed {}", a.luid);
            }
            let len = other.len();
            other.rotate_left(turn % len);
            if flip {
                other.reverse();
            }
            prop_assert_eq!(plan(&list), plan(&other));
        }
    }

    #[test]
    fn an_integrated_gpu_and_an_arc_gpu_try_the_arc_first_on_a_hybrid_laptop() {
        let laptop = [
            Adapter {
                display: true,
                power_saving: true,
                ..adapter(10, "Intel(R) Iris(R) Xe Graphics", VENDOR_INTEL)
            },
            Adapter {
                high_performance: true,
                ..adapter(20, "Intel(R) Arc(TM) A370M Graphics", VENDOR_INTEL)
            },
        ];
        assert_eq!(plan(&laptop), vec![hw(20), hw(10), sw(10)]);
    }

    #[test]
    fn two_gpus_with_the_same_name_are_told_apart_by_id() {
        let twins = [
            adapter(7, "Intel(R) UHD Graphics", VENDOR_INTEL),
            adapter(3, "Intel(R) UHD Graphics", VENDOR_INTEL),
        ];
        assert_eq!(plan(&twins), vec![hw(3), hw(7), sw(3)]);
    }

    #[test]
    fn a_gpu_that_fails_device_creation_is_skipped_for_the_next_one() {
        let failed = Attempt {
            adapter: "Intel(R) UHD Graphics".to_owned(),
            ..attempt(
                EncoderKind::Hardware,
                VENDOR_INTEL,
                Err("device creation failed"),
            )
        };
        let worked = Attempt {
            adapter: "NVIDIA GeForce RTX 3070".to_owned(),
            ..attempt(EncoderKind::Hardware, VENDOR_NVIDIA, Ok("ok"))
        };
        let check = EncoderCheck::from_attempts("key".to_owned(), &[failed, worked]);
        assert_eq!(check.outcome, CheckOutcome::Hardware);
        assert_eq!(check.encoder, "NVIDIA NVENC (hardware)");
    }

    #[test]
    fn worker_threads_follow_the_core_count() {
        for (cores, threads) in [(0, 2), (1, 2), (2, 2), (4, 2), (8, 4), (64, 4)] {
            assert_eq!(worker_threads(cores), threads, "{cores} cores");
        }
    }
}
