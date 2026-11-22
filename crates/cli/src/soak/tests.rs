use super::check::{Limits, Run, evaluate, growth, slope_per_min};
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
        },
        &limits(),
    )
}

fn args(cycles: u32, out: PathBuf, dir: Option<PathBuf>) -> Args {
    Args {
        cycles,
        seconds: 1,
        gap_secs: 0,
        game_minutes: None,
        out,
        dir,
        keep: false,
        warmup_secs: 0.0,
        warmup_cycles: 0,
        max_mem_mb_per_min: 2.0,
        max_gpu_mb_per_min: 2.0,
        max_handle_growth: 50,
        max_thread_growth: 4,
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
