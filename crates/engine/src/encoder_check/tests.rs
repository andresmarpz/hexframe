use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crossbeam_channel::Sender;
use greyscreen_core::encoder::{Attempt, EncoderKind, Probe, Reason, SOFTWARE_NOTICE, Verdict};
use greyscreen_core::media::{CaptureTarget, MediaEvent, MediaSession};
use greyscreen_core::settings::Settings;

use super::*;

const NVIDIA: u32 = 0x10de;

struct Encoders {
    fingerprint: Mutex<String>,
    results: Mutex<Vec<Attempt>>,
    checks: AtomicU32,
}

impl Encoders {
    fn new(results: Vec<Attempt>) -> Arc<Self> {
        Arc::new(Self {
            fingerprint: Mutex::new("NVIDIA 1.0".into()),
            results: Mutex::new(results),
            checks: AtomicU32::new(0),
        })
    }

    fn set(&self, results: Vec<Attempt>) {
        *self.results.lock().unwrap() = results;
    }

    fn checks(&self) -> u32 {
        self.checks.load(Ordering::SeqCst)
    }
}

impl MediaSource for Encoders {
    fn start(
        &self,
        _target: CaptureTarget,
        _cfg: CaptureConfig,
        _out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        Err("no capture in this test".into())
    }

    fn encoder_fingerprint(&self) -> Option<String> {
        Some(self.fingerprint.lock().unwrap().clone())
    }

    fn encoder_probes(&self) -> Vec<Probe> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        self.results
            .lock()
            .unwrap()
            .iter()
            .map(|a| probe(a.kind, &a.adapter))
            .collect()
    }

    fn probe_encoder(&self, _cfg: &CaptureConfig, probe: &Probe) -> Result<String, String> {
        let results = self.results.lock().unwrap();
        let attempt = results.iter().find(|a| a.kind == probe.kind).unwrap();
        attempt.result.clone()
    }
}

fn hardware(result: Result<&str, &str>) -> Attempt {
    Attempt {
        kind: EncoderKind::Hardware,
        adapter: "NVIDIA GeForce RTX 3070".into(),
        vendor_id: NVIDIA,
        result: result.map(str::to_owned).map_err(str::to_owned),
        reason: Reason::Failed,
    }
}

fn software(result: Result<&str, &str>) -> Attempt {
    Attempt {
        kind: EncoderKind::Software,
        ..hardware(result)
    }
}

fn cache_path() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "greyscreen-encoder-check-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::remove_dir_all(&dir).ok();
    dir.join("cache").join("encoder-check.json")
}

fn spec(media: &Arc<Encoders>, cache: &Path) -> EncoderCheckSpec {
    EncoderCheckSpec {
        media: media.clone(),
        cfg: Settings::default().capture_config(1080),
        cache: cache.to_path_buf(),
        version: "0.1".into(),
        delay: Duration::ZERO,
    }
}

fn run(spec: &EncoderCheckSpec) -> (Option<EncoderCheck>, Vec<UiMsg>) {
    let sent = Mutex::new(Vec::new());
    let check = run_encoder_check(spec, &|msg| sent.lock().unwrap().push(msg));
    (check, sent.into_inner().unwrap())
}

fn warnings(msgs: &[UiMsg]) -> usize {
    msgs.iter()
        .filter(|m| {
            matches!(m, UiMsg::NoticeWithAction { level: NoticeLevel::Warn, text, .. } if text == SOFTWARE_NOTICE)
        })
        .count()
}

#[test]
fn a_working_hardware_encoder_is_reported_without_a_notice() {
    let media = Encoders::new(vec![hardware(Ok("NVIDIA MFT"))]);
    let (check, msgs) = run(&spec(&media, &cache_path()));
    let check = check.unwrap();
    assert_eq!(check.outcome, CheckOutcome::Hardware);
    assert_eq!(msgs, vec![UiMsg::Encoder(check)]);
}

#[test]
fn the_result_is_cached_until_the_driver_changes() {
    let cache = cache_path();
    let media = Encoders::new(vec![hardware(Ok("NVIDIA MFT"))]);
    run(&spec(&media, &cache));
    run(&spec(&media, &cache));
    assert_eq!(media.checks(), 1);
    *media.fingerprint.lock().unwrap() = "NVIDIA 2.0".into();
    run(&spec(&media, &cache));
    assert_eq!(media.checks(), 2);
}

#[test]
fn a_busy_software_outcome_is_not_cached_and_says_who_to_close() {
    use greyscreen_core::encoder::{BUSY_HELP, OpenFailure};
    let cache = cache_path();
    let busy = OpenFailure::Busy.tag("SetOutputType failed");
    let media = Encoders::new(vec![hardware(Err(&busy)), software(Ok("Microsoft MFT"))]);
    let (check, msgs) = run(&spec(&media, &cache));
    let check = check.unwrap();
    assert_eq!(check.outcome, CheckOutcome::Software);
    assert_eq!(reasons(&check)[0].1, Some(Reason::Busy));
    assert!(msgs.iter().any(
        |m| matches!(m, UiMsg::NoticeWithAction { level: NoticeLevel::Warn, text, .. } if text == BUSY_HELP)
    ));
    run(&spec(&media, &cache));
    assert_eq!(media.checks(), 2);
}

#[test]
fn a_software_fallback_is_noticed_once() {
    let cache = cache_path();
    let media = Encoders::new(vec![hardware(Err("E_FAIL")), software(Ok("Microsoft MFT"))]);
    let (check, first) = run(&spec(&media, &cache));
    assert_eq!(check.unwrap().outcome, CheckOutcome::Software);
    assert_eq!(warnings(&first), 1);
    let (_, second) = run(&spec(&media, &cache));
    assert_eq!(warnings(&second), 0);
    assert_eq!(media.checks(), 1);
}

#[test]
fn a_failure_is_reported_and_checked_again_next_time() {
    let cache = cache_path();
    let media = Encoders::new(vec![hardware(Err("E_FAIL")), software(Err("E_FAIL"))]);
    let (check, msgs) = run(&spec(&media, &cache));
    assert!(check.unwrap().failed());
    assert_eq!(warnings(&msgs), 0);
    media.set(vec![hardware(Ok("NVIDIA MFT"))]);
    let (check, _) = run(&spec(&media, &cache));
    assert_eq!(check.unwrap().outcome, CheckOutcome::Hardware);
    assert_eq!(media.checks(), 2);
}

#[test]
fn a_source_without_encoders_skips_the_check() {
    let cache = cache_path();
    let spec = EncoderCheckSpec {
        media: Arc::new(crate::FakeMediaSource::fast()),
        ..spec(&Encoders::new(Vec::new()), &cache)
    };
    let (check, msgs) = run(&spec);
    assert!(check.is_none());
    assert!(msgs.is_empty());
    assert!(!cache.exists());
}

#[test]
fn an_unreadable_cache_runs_the_check_again() {
    let cache = cache_path();
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::write(&cache, "not json").unwrap();
    let media = Encoders::new(vec![hardware(Ok("NVIDIA MFT"))]);
    let (check, _) = run(&spec(&media, &cache));
    assert_eq!(check.unwrap().outcome, CheckOutcome::Hardware);
    assert_eq!(media.checks(), 1);
}

fn probe(kind: EncoderKind, adapter: &str) -> Probe {
    Probe {
        kind,
        adapter: adapter.into(),
        vendor_id: NVIDIA,
        driver_version: "1.0".into(),
        luid: 0,
    }
}

#[derive(Clone, Copy)]
enum Behavior {
    Pass,
    Panic,
    Silent,
    Hang,
    Slow,
}

const DEADLINE: Duration = Duration::from_millis(400);

struct Fake {
    plan: Vec<(Probe, Behavior)>,
    runs: Mutex<Vec<String>>,
}

impl Fake {
    fn new(plan: Vec<(&str, EncoderKind, Behavior)>) -> Arc<Self> {
        Arc::new(Self {
            plan: plan
                .into_iter()
                .map(|(name, kind, behavior)| (probe(kind, name), behavior))
                .collect(),
            runs: Mutex::new(Vec::new()),
        })
    }

    fn runs_of(&self, name: &str) -> usize {
        self.runs
            .lock()
            .unwrap()
            .iter()
            .filter(|r| *r == name)
            .count()
    }
}

impl ProbeRunner for Fake {
    fn probes(&self) -> Vec<Probe> {
        self.plan.iter().map(|(probe, _)| probe.clone()).collect()
    }

    fn run(&self, _cfg: &CaptureConfig, probe: &Probe) -> ProbeResult {
        self.runs.lock().unwrap().push(probe.adapter.clone());
        let behavior = self
            .plan
            .iter()
            .find(|(p, _)| p.same_target(probe))
            .map(|(_, b)| *b)
            .unwrap();
        match behavior {
            Behavior::Pass => ProbeResult::Passed("ok".into()),
            Behavior::Panic => panic!("driver crashed"),
            Behavior::Silent => ProbeResult::Crashed("the probe printed nothing".into()),
            Behavior::Hang => {
                std::thread::sleep(Duration::from_secs(3));
                ProbeResult::Passed("too late".into())
            }
            Behavior::Slow => {
                std::thread::sleep(DEADLINE / 4);
                ProbeResult::Passed("slow but fine".into())
            }
        }
    }
}

fn run_fake(fake: &Arc<Fake>, spec: &EncoderCheckSpec) -> (EncoderCheck, Vec<UiMsg>) {
    let runner: Arc<dyn ProbeRunner> = fake.clone();
    let sent = Mutex::new(Vec::new());
    let check = run_with_runner(spec, &runner, DEADLINE, &|msg| {
        sent.lock().unwrap().push(msg)
    });
    (check.unwrap(), sent.into_inner().unwrap())
}

fn reasons(check: &EncoderCheck) -> Vec<(String, Option<Reason>)> {
    check
        .verdicts
        .iter()
        .map(|v| (v.probe.adapter.clone(), v.reason))
        .collect()
}

use EncoderKind::{Hardware, Software};

fn with_fingerprint(cache: &Path) -> EncoderCheckSpec {
    spec(&Encoders::new(vec![hardware(Ok("x"))]), cache)
}

#[test]
fn a_crashing_probe_marks_only_its_adapter() {
    for behavior in [Behavior::Panic, Behavior::Silent] {
        let fake = Fake::new(vec![
            ("A", Hardware, behavior),
            ("B", Hardware, Behavior::Pass),
        ]);
        let (check, _) = run_fake(&fake, &with_fingerprint(&cache_path()));
        assert_eq!(check.outcome, CheckOutcome::Hardware);
        assert_eq!(
            reasons(&check),
            vec![("A".into(), Some(Reason::Crashed)), ("B".into(), None)]
        );
    }
}

#[test]
fn a_hanging_probe_times_out_and_is_not_a_missing_encoder() {
    let fake = Fake::new(vec![("A", Hardware, Behavior::Hang)]);
    let started = std::time::Instant::now();
    let (check, _) = run_fake(&fake, &with_fingerprint(&cache_path()));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(check.failed());
    assert_eq!(reasons(&check), vec![("A".into(), Some(Reason::TimedOut))]);
    assert!(check.detail.contains("no answer"));
}

#[test]
fn a_slow_probe_inside_the_deadline_passes() {
    let fake = Fake::new(vec![("A", Hardware, Behavior::Slow)]);
    let (check, _) = run_fake(&fake, &with_fingerprint(&cache_path()));
    assert_eq!(check.outcome, CheckOutcome::Hardware);
    assert_eq!(reasons(&check), vec![("A".into(), None)]);
}

#[test]
fn a_cached_crashed_adapter_is_skipped_until_its_driver_changes() {
    let cache = cache_path();
    let fake = Fake::new(vec![
        ("A", Hardware, Behavior::Panic),
        ("B", Hardware, Behavior::Pass),
    ]);
    run_fake(&fake, &with_fingerprint(&cache));
    assert_eq!(fake.runs_of("A"), 1);
    let mut spec = with_fingerprint(&cache);
    spec.version = "0.2".into();
    let (check, _) = run_fake(&fake, &spec);
    assert_eq!(fake.runs_of("A"), 1);
    assert_eq!(check.outcome, CheckOutcome::Hardware);
    let mut newer = Fake::new(vec![("A", Hardware, Behavior::Pass)]);
    Arc::get_mut(&mut newer).unwrap().plan[0].0.driver_version = "2.0".into();
    spec.version = "0.3".into();
    let (check, _) = run_fake(&newer, &spec);
    assert_eq!(newer.runs_of("A"), 1);
    assert_eq!(reasons(&check), vec![("A".into(), None)]);
}

#[test]
fn a_leftover_marker_counts_as_a_crash_on_its_adapter() {
    let cache = cache_path();
    let marker = marker_path(&cache);
    std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
    let leftover = serde_json::to_string(&probe(Hardware, "A")).unwrap();
    std::fs::write(&marker, leftover).unwrap();
    let fake = Fake::new(vec![
        ("A", Hardware, Behavior::Pass),
        ("B", Hardware, Behavior::Pass),
    ]);
    let (check, _) = run_fake(&fake, &with_fingerprint(&cache));
    assert_eq!(fake.runs_of("A"), 0);
    assert_eq!(
        reasons(&check),
        vec![("A".into(), Some(Reason::Crashed)), ("B".into(), None)]
    );
    assert!(!marker.exists());
}

#[test]
fn the_marker_is_removed_after_each_probe() {
    let cache = cache_path();
    let fake = Fake::new(vec![("A", Hardware, Behavior::Pass)]);
    run_fake(&fake, &with_fingerprint(&cache));
    assert!(!marker_path(&cache).exists());
}

#[test]
fn a_software_outcome_caused_by_a_crash_or_timeout_is_retried_next_launch() {
    for behavior in [Behavior::Panic, Behavior::Hang] {
        let cache = cache_path();
        let fake = Fake::new(vec![
            ("A", Hardware, behavior),
            ("S", Software, Behavior::Pass),
        ]);
        let (first, msgs) = run_fake(&fake, &with_fingerprint(&cache));
        assert_eq!(first.outcome, CheckOutcome::Software);
        assert_eq!(warnings(&msgs), 1);
        let (_, msgs) = run_fake(&fake, &with_fingerprint(&cache));
        assert_eq!(fake.runs_of("S"), 2);
        assert_eq!(warnings(&msgs), 0);
    }
}

type Change = Box<dyn Fn(&mut EncoderCheckSpec)>;

#[test]
fn a_changed_setting_or_version_runs_the_check_again() {
    let changes: Vec<Change> = vec![
        Box::new(|s| s.cfg.fps += 30),
        Box::new(|s| s.cfg.max_height = Some(720)),
        Box::new(|s| s.cfg.quality = Some(80)),
        Box::new(|s| s.version = "9.9".into()),
    ];
    for change in &changes {
        let cache = cache_path();
        let media = Encoders::new(vec![hardware(Ok("NVIDIA MFT"))]);
        run(&spec(&media, &cache));
        run(&spec(&media, &cache));
        assert_eq!(media.checks(), 1);
        let mut changed = spec(&media, &cache);
        change(&mut changed);
        run(&changed);
        assert_eq!(media.checks(), 2);
    }
}

#[test]
fn abandoning_the_check_removes_the_marker() {
    let cache = cache_path();
    let marker = marker_path(&cache);
    std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
    std::fs::write(&marker, "{}").unwrap();
    abandon_encoder_check(&cache);
    assert!(!marker.exists());
    abandon_encoder_check(&cache);
}

#[test]
fn the_check_waits_for_its_delay() {
    let delay = Duration::from_millis(300);
    let media = Encoders::new(vec![hardware(Ok("NVIDIA MFT"))]);
    let spec = EncoderCheckSpec {
        delay,
        ..spec(&media, &cache_path())
    };
    let (tx, rx) = crossbeam_channel::unbounded();
    let started = std::time::Instant::now();
    let handle = spawn_encoder_check(spec, Box::new(move |msg| tx.send(msg).unwrap())).unwrap();
    assert!(rx.recv().is_ok());
    assert!(started.elapsed() >= delay);
    handle.join().unwrap();
}

#[test]
fn verdicts_are_saved_in_the_cache() {
    let cache = cache_path();
    let fake = Fake::new(vec![
        ("A", Hardware, Behavior::Panic),
        ("B", Hardware, Behavior::Pass),
    ]);
    run_fake(&fake, &with_fingerprint(&cache));
    let saved = load(&cache).unwrap();
    let verdict = |name, reason| Verdict {
        probe: probe(Hardware, name),
        reason,
    };
    assert_eq!(
        saved.verdicts,
        vec![verdict("A", Some(Reason::Crashed)), verdict("B", None)]
    );
}

#[cfg(unix)]
mod child {
    use greyscreen_core::encoder::PROBE_FLAG;

    use super::*;

    const CHILD_DEADLINE: Duration = Duration::from_millis(500);

    fn runner(script: &str) -> Arc<ChildRunner> {
        let media = Encoders::new(vec![hardware(Ok("x")), software(Ok("x"))]);
        Arc::new(ChildRunner::with_args(
            media,
            "sh".into(),
            vec!["-c".into(), script.into()],
            CHILD_DEADLINE,
        ))
    }

    fn run_script(script: &str) -> ProbeResult {
        let cfg = Settings::default().capture_config(1080);
        runner(script).run(&cfg, &probe(Hardware, "A"))
    }

    #[test]
    fn a_result_line_is_read_from_the_child() {
        assert_eq!(
            run_script("echo noise; echo 'result=ok detail=NVENC'"),
            ProbeResult::Passed("NVENC".into())
        );
        assert_eq!(
            run_script("echo 'result=failed detail=no device'; exit 1"),
            ProbeResult::Failed("no device".into())
        );
    }

    #[test]
    fn an_abnormal_exit_or_empty_output_is_a_crash() {
        assert!(matches!(run_script("kill -9 $$"), ProbeResult::Crashed(_)));
        assert!(matches!(run_script("exit 0"), ProbeResult::Crashed(_)));
        assert!(matches!(run_script("exit 7"), ProbeResult::Crashed(_)));
    }

    #[test]
    fn a_child_that_hangs_is_stopped_at_the_deadline_and_times_out() {
        let started = std::time::Instant::now();
        let result = run_script("exec sleep 30");
        assert!(matches!(result, ProbeResult::TimedOut(_)));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_missing_program_fails_without_taking_the_parent_down() {
        let media = Encoders::new(vec![hardware(Ok("x"))]);
        let runner = ChildRunner::new(media, "/nonexistent/greyscreen".into(), CHILD_DEADLINE);
        let cfg = Settings::default().capture_config(1080);
        assert!(matches!(
            runner.run(&cfg, &probe(Hardware, "A")),
            ProbeResult::Failed(_)
        ));
    }

    #[test]
    fn the_parent_survives_a_crashed_child_and_runs_the_next_candidate() {
        let script = r#"case "$0" in *'"kind":"hardware"'*) kill -9 $$;; *) echo 'result=ok detail=software';; esac"#;
        let runner: Arc<dyn ProbeRunner> = runner(script);
        let media = Encoders::new(vec![hardware(Ok("x")), software(Ok("x"))]);
        let check = run_with_runner(
            &spec(&media, &cache_path()),
            &runner,
            CHILD_DEADLINE + Duration::from_secs(3),
            &|_| {},
        )
        .unwrap();
        assert_eq!(check.outcome, CheckOutcome::Software);
        assert_eq!(reasons(&check)[0].1, Some(Reason::Crashed));
    }

    #[test]
    fn the_child_entry_point_ignores_a_normal_launch_and_rejects_a_bad_request() {
        let media = Encoders::new(vec![]);
        assert_eq!(run_probe_child(&[], &*media), None);
        assert_eq!(run_probe_child(&["--hidden".into()], &*media), None);
        let bad = [PROBE_FLAG.to_owned(), "nonsense".to_owned()];
        assert_eq!(run_probe_child(&bad, &*media), Some(2));
    }
}
