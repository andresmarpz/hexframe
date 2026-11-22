use super::check::{Limits, Run, evaluate, growth, slope_per_min};
use super::profiles::WindowTarget;
use super::*;

const MB: u64 = 1024 * 1024;

fn limits() -> Limits {
    Limits {
        mem_mb_per_min: 2.0,
        gpu_mb_per_min: 2.0,
        handle_growth: 50,
        thread_growth: 4,
        warmup_secs: 10.0,
        warmup_cycles: 1,
        max_dropped_pct: None,
        encoder_contains: None,
        av: None,
    }
}

fn samples(count: u32, mem: impl Fn(u32) -> u64, handles: impl Fn(u32) -> u32) -> Vec<Sample> {
    (0..count)
        .map(|t| Sample {
            t_secs: f64::from(t),
            recording: false,
            checkpoint: t % 10 == 0,
            private_bytes: Some(mem(t)),
            handles: Some(handles(t)),
            threads: Some(40),
            gpu_bytes: Some(300 * MB),
        })
        .collect()
}

fn good_cycle(index: u32) -> CycleResult {
    CycleResult {
        index,
        folder: String::new(),
        video_secs: 30.0,
        dropped_pct: 0.0,
        capture_errors: 0,
        problem: None,
        health: None,
        diag_seconds: Vec::new(),
    }
}

fn verdict(samples: &[Sample], cycles: &[CycleResult], errors: &[String]) -> Verdict {
    evaluate(
        &Run {
            samples,
            cycles,
            engine_errors: errors,
            min_saved: 2,
            require_usage: true,
            by_checkpoint: false,
            device_removed: 0,
        },
        &limits(),
    )
}

fn args(cycles: u32, out: PathBuf, dir: Option<PathBuf>) -> Args {
    Args {
        profile: None,
        list_profiles: false,
        window: WindowTarget::League,
        cycles,
        seconds: 1,
        gap_secs: 0,
        game_minutes: None,
        out: Some(out),
        dir,
        keep: false,
        d3d_debug: false,
        warmup_secs: 0.0,
        warmup_cycles: 0,
        max_mem_mb_per_min: 2.0,
        max_gpu_mb_per_min: 2.0,
        max_handle_growth: 50,
        max_thread_growth: 4,
        max_dropped_pct: None,
        encoder_contains: None,
        min_video_secs: None,
        until_saved: false,
        av_limits: None,
        random_secs: None,
        failed_clip_saves: false,
    }
}

#[test]
fn slope_of_a_line_is_exact_per_minute() {
    let points: Vec<(f64, f64)> = (0..100)
        .map(|t| (f64::from(t), 5.0 + f64::from(t) * 0.5))
        .collect();
    assert!((slope_per_min(&points).unwrap() - 30.0).abs() < 1e-9);
    assert_eq!(slope_per_min(&[(1.0, 1.0)]), None);
    assert_eq!(slope_per_min(&[(1.0, 1.0), (1.0, 2.0)]), None);
}

#[test]
fn growth_compares_the_end_against_the_start_and_ignores_one_spike() {
    let mut values = vec![100.0; 40];
    values[20] = 9000.0;
    assert_eq!(growth(&values), Some(0.0));
    values.extend([140.0; 10]);
    assert!(growth(&values).unwrap() > 30.0);
}

#[test]
fn flat_run_passes() {
    let run = samples(120, |t| 500 * MB + u64::from(t % 3) * MB, |_| 600);
    let cycles = [good_cycle(1), good_cycle(2)];
    let v = verdict(&run, &cycles, &[]);
    assert!(v.passed, "{:?}", v.failures);
}

#[test]
fn leaking_memory_fails() {
    let run = samples(120, |t| 500 * MB + u64::from(t) * MB, |_| 600);
    let cycles = [good_cycle(1), good_cycle(2)];
    let v = verdict(&run, &cycles, &[]);
    assert!(!v.passed);
    assert!(v.failures[0].starts_with("private memory"));
}

#[test]
fn warmup_growth_is_not_counted() {
    let run = samples(120, |t| 500 * MB + u64::from(t.min(10)) * 50 * MB, |_| 600);
    let cycles = [good_cycle(1), good_cycle(2)];
    assert!(verdict(&run, &cycles, &[]).passed);
}

#[test]
fn handle_growth_fails() {
    let run = samples(120, |_| 500 * MB, |t| 600 + t * 2);
    let cycles = [good_cycle(1), good_cycle(2)];
    let v = verdict(&run, &cycles, &[]);
    assert!(v.failures.iter().any(|f| f.starts_with("handles")));
}

#[test]
fn missing_recordings_bad_files_and_engine_errors_fail() {
    let run = samples(120, |_| 500 * MB, |_| 600);
    let mut bad = good_cycle(2);
    bad.problem = Some("video is 0.0 s".to_owned());
    bad.capture_errors = 1;
    let v = verdict(&run, &[bad], &["device lost".to_owned()]);
    assert_eq!(v.failures.len(), 4, "{:?}", v.failures);
}

#[test]
fn a_short_run_is_noted_and_unmeasured_memory_fails_when_required() {
    let short = samples(14, |t| 500 * MB + u64::from(t) * 100 * MB, |_| 600);
    let cycles = [good_cycle(1), good_cycle(2)];
    let v = verdict(&short, &cycles, &[]);
    assert!(v.passed);
    assert_eq!(v.notes.len(), 4, "{:?}", v.notes);
    let empty: Vec<Sample> = (0..60)
        .map(|t| Sample {
            t_secs: f64::from(t),
            ..Sample::default()
        })
        .collect();
    assert!(!verdict(&empty, &cycles, &[]).passed);
}

#[test]
fn recording_phase_sawtooth_is_not_a_leak_but_a_rising_idle_floor_is() {
    let saw = |floor: u64| -> Vec<Sample> {
        (0..400u32)
            .map(|t| {
                let phase = t % 40;
                let recording = phase < 30;
                Sample {
                    t_secs: f64::from(t),
                    recording,
                    checkpoint: phase == 39,
                    private_bytes: Some(
                        (500 + floor * u64::from(t / 40)
                            + if recording { u64::from(phase) * 3 } else { 0 })
                            * MB,
                    ),
                    handles: Some(if recording { 700 } else { 600 }),
                    threads: Some(if recording { 48 } else { 40 }),
                    gpu_bytes: Some(300 * MB),
                }
            })
            .collect()
    };
    let cycles = [good_cycle(1), good_cycle(2)];
    for by_checkpoint in [true, false] {
        let judge = |samples: &[Sample]| {
            evaluate(
                &Run {
                    samples,
                    cycles: &cycles,
                    engine_errors: &[],
                    min_saved: 2,
                    require_usage: true,
                    by_checkpoint,
                    device_removed: 0,
                },
                &limits(),
            )
        };
        assert!(judge(&saw(0)).passed, "{by_checkpoint}");
        assert!(!judge(&saw(30)).passed, "{by_checkpoint}");
    }
}

#[test]
fn a_dead_engine_still_writes_a_failing_report() {
    let dir = std::env::temp_dir().join(format!("greyscreen-soak-dead-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("report.json");
    let args = args(2, out.clone(), Some(dir.join("rec")));
    let (_tx, rx) = unbounded::<UiMsg>();
    let (cmd, cmd_rx) = unbounded();
    drop(cmd_rx);
    let mut soak = Soak::new(&args, rx);
    soak.run_cycles(&cmd);
    let verdict = soak.finish().unwrap();
    assert!(!verdict.passed);
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(report["engine_errors"][0], "engine stopped");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn two_engine_cycles_with_the_fake_source_pass_and_write_the_report() {
    let dir = std::env::temp_dir().join(format!("greyscreen-soak-test-{}", std::process::id()));
    let out = dir.join("report.json");
    let args = args(2, out.clone(), Some(dir.join("rec")));
    std::fs::create_dir_all(&dir).unwrap();
    let verdict = run_with(
        &args,
        Arc::new(greyscreen_engine::FakeMediaSource::new()),
        default_league(&args),
    )
    .unwrap();
    assert!(verdict.passed, "{:?}", verdict.failures);
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(report["cycles"].as_array().unwrap().len(), 2);
    assert_eq!(report["verdict"]["passed"], true);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn zero_cycles_are_rejected() {
    assert!(args(0, PathBuf::from("x.json"), None).validate().is_err());
}

fn with_profile(name: &str) -> Args {
    Args {
        profile: Some(name.to_owned()),
        out: None,
        ..args(8, PathBuf::from("x.json"), None)
    }
}

#[test]
fn every_profile_resolves_to_valid_args_with_unique_names() {
    let mut names = std::collections::HashSet::new();
    for profile in profiles::PROFILES {
        assert!(names.insert(profile.name), "duplicate {}", profile.name);
        let resolved = with_profile(profile.name).resolve().unwrap();
        resolved.validate().unwrap();
        match profile.shape {
            profiles::Shape::Games { minutes } => {
                assert_eq!(resolved.game_minutes, Some(minutes));
                assert!(profile.uses_window);
            }
            profiles::Shape::RandomCycles {
                cycles, max_secs, ..
            } => {
                assert_eq!((resolved.cycles, resolved.seconds), (cycles, max_secs));
                assert!(resolved.random_secs.is_some());
            }
            profiles::Shape::FailedClips { cycles, seconds } => {
                assert_eq!((resolved.cycles, resolved.seconds), (cycles, seconds));
                assert!(resolved.failed_clip_saves);
            }
            profiles::Shape::Cycles {
                cycles, seconds, ..
            } => {
                assert_eq!((resolved.cycles, resolved.seconds), (cycles, seconds));
                assert_eq!(resolved.game_minutes, None);
            }
        }
    }
}

#[test]
fn the_three_hour_profile_waits_for_a_recording_that_reaches_the_limit() {
    let resolved = with_profile("av-180").resolve().unwrap();
    assert!(resolved.until_saved);
    assert!(resolved.game_minutes.unwrap() >= 200);
    assert!(resolved.min_video_secs.unwrap() > 10_000.0);
}

#[test]
fn the_overnight_profile_needs_no_one_at_the_pc() {
    let resolved = with_profile("long-8h").resolve().unwrap();
    assert_eq!(resolved.game_minutes, None);
    assert!(resolved.cycles >= 3);
    assert!(u64::from(resolved.cycles) * u64::from(resolved.seconds) >= 8 * 3600 - 3600);
}

#[test]
fn a_profile_names_its_report_unless_out_is_given() {
    let resolved = with_profile("smoke").resolve().unwrap();
    assert_eq!(
        resolved.out_path(),
        PathBuf::from("greyscreen-soak-smoke.json")
    );
    let mut given = with_profile("smoke");
    given.out = Some(PathBuf::from("mine.json"));
    assert_eq!(
        given.resolve().unwrap().out_path(),
        PathBuf::from("mine.json")
    );
}

#[test]
fn drops_and_the_wrong_encoder_fail_when_the_limits_say_so() {
    let mut strict = limits();
    strict.max_dropped_pct = Some(0.0);
    strict.encoder_contains = Some("Microsoft H.264");
    let samples = samples(60, |_| 100 * MB, |_| 200);
    let mut cycle = good_cycle(1);
    cycle.dropped_pct = 0.5;
    cycle.health = Some(greyscreen_core::health::RecordingHealth {
        encoder: "NVIDIA H.264 Encoder MFT".to_owned(),
        ..Default::default()
    });
    let cycles = [cycle, good_cycle(2)];
    let run = Run {
        samples: &samples,
        cycles: &cycles,
        engine_errors: &[],
        min_saved: 2,
        require_usage: true,
        by_checkpoint: false,
        device_removed: 0,
    };
    let failures = evaluate(&run, &strict).failures;
    assert!(
        failures.iter().any(|f| f.contains("dropped 0.50%")),
        "{failures:?}"
    );
    assert_eq!(failures.iter().filter(|f| f.contains("encoder")).count(), 2);
}

#[test]
fn unknown_profiles_and_the_unbuilt_cloaked_window_are_rejected() {
    assert!(with_profile("nope").resolve().is_err());
    let mut cloaked = with_profile("game-30");
    cloaked.window = WindowTarget::Cloaked;
    assert!(cloaked.resolve().is_err());
    cloaked.profile = Some("smoke".to_owned());
    assert!(cloaked.resolve().is_ok());
}

#[test]
fn a_removed_device_fails_the_run_and_the_debug_layer_notes_its_reports() {
    let samples = samples(60, |_| 100 * MB, |_| 200);
    let cycles = [good_cycle(1), good_cycle(2)];
    let run = |device_removed| Run {
        samples: &samples,
        cycles: &cycles,
        engine_errors: &[],
        min_saved: 2,
        require_usage: true,
        by_checkpoint: false,
        device_removed,
    };
    assert!(evaluate(&run(0), &limits()).passed);
    let failures = evaluate(&run(2), &limits()).failures;
    assert!(
        failures.iter().any(|f| f.contains("removed 2 times")),
        "{failures:?}"
    );

    let mut gpu = gpu::GpuReport::default();
    assert!(gpu.notes().is_empty());
    gpu.debug_layer = true;
    assert!(gpu.notes()[0].contains("no live object report"));
    gpu.live_objects = vec![gpu::LiveObjects {
        count: 3,
        lines: Vec::new(),
    }];
    assert!(gpu.notes()[0].contains(": 3 "));
}

fn av_cycle(skews: &[f64], video_secs: f64) -> CycleResult {
    CycleResult {
        video_secs,
        diag_seconds: skews
            .iter()
            .enumerate()
            .map(|(t, s)| serde_json::json!({"type": "second", "t": t as f64, "av_skew_ms": s}))
            .collect(),
        ..good_cycle(1)
    }
}

fn alternating(a: f64, b: f64) -> Vec<f64> {
    (0..3600).map(|t| if t % 2 == 0 { a } else { b }).collect()
}

#[test]
fn av_limits_judge_skew_and_drift_per_hour() {
    let mut strict = limits();
    strict.av = Some(check::AvLimits {
        max_skew_ms: 40.0,
        max_drift_ms_per_hour: 1.0,
    });
    let samples = samples(60, |_| 100 * MB, |_| 200);
    let failures = |cycle: CycleResult| {
        let cycles = [cycle, good_cycle(2)];
        let run = Run {
            samples: &samples,
            cycles: &cycles,
            engine_errors: &[],
            min_saved: 2,
            require_usage: true,
            by_checkpoint: false,
            device_removed: 0,
        };
        evaluate(&run, &strict).failures
    };
    assert!(failures(av_cycle(&alternating(5.0, -39.0), 7200.0)).is_empty());
    let skewed = failures(av_cycle(&alternating(5.0, -41.0), 7200.0));
    assert!(
        skewed.iter().any(|f| f.contains("skew reached 41.0")),
        "{skewed:?}"
    );
    let steady_offset = vec![12.0; 3600];
    assert!(failures(av_cycle(&steady_offset, 3600.0)).is_empty());
    let sloped: Vec<f64> = (0..3600).map(|t| -f64::from(t) * 2.0 / 3600.0).collect();
    let drifted = failures(av_cycle(&sloped, 3600.0));
    assert!(
        drifted.iter().any(|f| f.contains("2.00 ms per hour")),
        "{drifted:?}"
    );
    assert!(failures(av_cycle(&sloped, 60.0)).is_empty());
}

#[test]
fn the_av_profiles_carry_the_limits_and_random_draws_stay_in_range() {
    assert!(with_profile("av-60").resolve().unwrap().av_limits.is_some());
    assert!(with_profile("smoke").resolve().unwrap().av_limits.is_none());
    let mut rng = rng::Rng::new(42);
    for _ in 0..1000 {
        assert!((1..=8).contains(&rng.between(1, 8)));
    }
    assert_eq!(rng.between(3, 3), 3);
}

#[test]
fn random_cycles_cut_recordings_short_and_still_save_each_one() {
    let dir = std::env::temp_dir().join(format!("greyscreen-soak-rand-{}", std::process::id()));
    let out = dir.join("report.json");
    let mut args = args(2, out.clone(), Some(dir.join("rec")));
    args.seconds = 3;
    args.random_secs = Some((1, 2));
    std::fs::create_dir_all(&dir).unwrap();
    let verdict = run_with(
        &args,
        Arc::new(greyscreen_engine::FakeMediaSource::new()),
        default_league(&args),
    )
    .unwrap();
    assert!(verdict.passed, "{:?}", verdict.failures);
    assert!(verdict.notes.iter().any(|n| n.starts_with("random seed")));
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let cycles = report["cycles"].as_array().unwrap();
    assert_eq!(cycles.len(), 2);
    for cycle in cycles {
        let secs = cycle["video_secs"].as_f64().unwrap();
        assert!(secs > 0.0 && secs < f64::from(args.seconds), "{secs}");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn failed_clip_saves_are_counted_and_the_run_still_saves_each_recording() {
    let dir = std::env::temp_dir().join(format!("greyscreen-soak-clips-{}", std::process::id()));
    let out = dir.join("report.json");
    let mut args = args(2, out.clone(), Some(dir.join("rec")));
    args.seconds = 6;
    args.failed_clip_saves = true;
    std::fs::create_dir_all(&dir).unwrap();
    let verdict = run_with(
        &args,
        Arc::new(greyscreen_engine::FakeMediaSource::new()),
        default_league(&args),
    )
    .unwrap();
    assert!(verdict.passed, "{:?}", verdict.failures);
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(report["clip_saves_failed"], 2);
    assert_eq!(report["cycles"].as_array().unwrap().len(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_recording_saved_while_waiting_for_the_clip_failure_keeps_every_cycle() {
    let dir = std::env::temp_dir().join(format!("greyscreen-soak-early-{}", std::process::id()));
    let out = dir.join("report.json");
    let mut args = args(2, out.clone(), Some(dir.join("rec")));
    args.seconds = 1;
    args.failed_clip_saves = true;
    std::fs::create_dir_all(&dir).unwrap();
    let verdict = run_with(
        &args,
        Arc::new(greyscreen_engine::FakeMediaSource::new()),
        default_league(&args),
    )
    .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(
        report["cycles"].as_array().unwrap().len(),
        2,
        "{:?}",
        verdict.failures
    );
    assert!(
        verdict
            .failures
            .iter()
            .all(|f| !f.contains("no saved recording")),
        "{:?}",
        verdict.failures
    );
    std::fs::remove_dir_all(&dir).ok();
}
