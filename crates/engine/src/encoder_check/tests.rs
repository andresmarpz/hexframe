use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crossbeam_channel::Sender;
use greyscreen_core::encoder::{Attempt, EncoderKind};
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

    fn check_encoders(&self, _cfg: &CaptureConfig, _every: bool) -> Vec<Attempt> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        self.results.lock().unwrap().clone()
    }
}

fn hardware(result: Result<&str, &str>) -> Attempt {
    Attempt {
        kind: EncoderKind::Hardware,
        adapter: "NVIDIA GeForce RTX 3070".into(),
        vendor_id: NVIDIA,
        result: result.map(str::to_owned).map_err(str::to_owned),
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
