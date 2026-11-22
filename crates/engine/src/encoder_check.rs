use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use greyscreen_core::encoder::{self, Attempt, CheckOutcome, EncoderCheck, Probe, Reason, Verdict};
use greyscreen_core::media::{CaptureConfig, MediaSource};
use greyscreen_core::problem::Problem;

use crate::api::{NoticeAction, NoticeLevel, UiMsg};

mod child;
mod runner;

pub use child::{ChildRunner, run_probe_child};
use runner::run_with_deadline;
pub use runner::{InProcessRunner, ProbeResult, ProbeRunner};

const PROBE_DEADLINE: Duration = Duration::from_secs(10);
const CHILD_GRACE: Duration = Duration::from_secs(3);

pub struct EncoderCheckSpec {
    pub media: Arc<dyn MediaSource>,
    pub cfg: CaptureConfig,
    pub cache: PathBuf,
    pub version: String,
    pub delay: Duration,
}

impl std::fmt::Debug for EncoderCheckSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncoderCheckSpec")
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

pub fn spawn_encoder_check(
    spec: EncoderCheckSpec,
    sink: Box<dyn Fn(UiMsg) + Send>,
) -> Option<JoinHandle<()>> {
    spawn_with(spec, sink, run_encoder_check)
}

pub fn spawn_isolated_encoder_check(
    spec: EncoderCheckSpec,
    sink: Box<dyn Fn(UiMsg) + Send>,
) -> Option<JoinHandle<()>> {
    spawn_with(spec, sink, run_isolated_encoder_check)
}

fn spawn_with(
    spec: EncoderCheckSpec,
    sink: Box<dyn Fn(UiMsg) + Send>,
    run: fn(&EncoderCheckSpec, &dyn Fn(UiMsg)) -> Option<EncoderCheck>,
) -> Option<JoinHandle<()>> {
    let spawned = std::thread::Builder::new()
        .name("encoder-check".into())
        .spawn(move || {
            std::thread::sleep(spec.delay);
            run(&spec, &*sink);
        });
    match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::warn!(%err, "cannot start the encoder check");
            None
        }
    }
}

pub fn run_encoder_check(spec: &EncoderCheckSpec, sink: &dyn Fn(UiMsg)) -> Option<EncoderCheck> {
    let runner: Arc<dyn ProbeRunner> = Arc::new(InProcessRunner::new(spec.media.clone()));
    run_with_runner(spec, &runner, PROBE_DEADLINE, sink)
}

pub fn run_isolated_encoder_check(
    spec: &EncoderCheckSpec,
    sink: &dyn Fn(UiMsg),
) -> Option<EncoderCheck> {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            tracing::warn!(%err, "cannot find the app to run the encoder check in a child");
            return None;
        }
    };
    let runner: Arc<dyn ProbeRunner> =
        Arc::new(ChildRunner::new(spec.media.clone(), exe, PROBE_DEADLINE));
    run_with_runner(spec, &runner, PROBE_DEADLINE + CHILD_GRACE, sink)
}

fn run_with_runner(
    spec: &EncoderCheckSpec,
    runner: &Arc<dyn ProbeRunner>,
    deadline: Duration,
    sink: &dyn Fn(UiMsg),
) -> Option<EncoderCheck> {
    let fingerprint = spec.media.encoder_fingerprint()?;
    let key = encoder::check_key(&fingerprint, &spec.cfg, &spec.version);
    let previous = record_leftover_crash(spec, load(&spec.cache));
    let mut check = match previous {
        Some(cached) if cached.reusable_for(&key) => {
            tracing::info!(encoder = %cached.encoder, "encoder check cached");
            cached
        }
        previous => fresh(spec, runner, deadline, key, previous.as_ref()),
    };
    sink(UiMsg::Encoder(check.clone()));
    if check.wants_notice() {
        sink(UiMsg::NoticeWithAction {
            level: NoticeLevel::Warn,
            text: check.notice_text().to_owned(),
            action: NoticeAction::Link(Problem::NoEncoder.help_url()),
        });
        check.noticed = true;
        save(&spec.cache, &check);
    }
    Some(check)
}

fn marker_path(cache: &Path) -> PathBuf {
    cache.with_extension("running")
}

pub fn abandon_encoder_check(cache: &Path) {
    std::fs::remove_file(marker_path(cache)).ok();
}

fn record_leftover_crash(
    spec: &EncoderCheckSpec,
    previous: Option<EncoderCheck>,
) -> Option<EncoderCheck> {
    let marker = marker_path(&spec.cache);
    let Ok(text) = std::fs::read_to_string(&marker) else {
        return previous;
    };
    std::fs::remove_file(&marker).ok();
    let Ok(probe) = serde_json::from_str::<Probe>(&text) else {
        tracing::warn!("encoder check marker unreadable");
        return previous;
    };
    tracing::warn!(adapter = %probe.adapter, "the last encoder check crashed");
    let mut check = previous.unwrap_or_else(|| EncoderCheck {
        key: String::new(),
        outcome: CheckOutcome::Failed,
        encoder: String::new(),
        detail: String::new(),
        noticed: false,
        verdicts: Vec::new(),
    });
    check.verdicts.retain(|v| !v.probe.same_target(&probe));
    check.verdicts.push(Verdict {
        probe,
        reason: Some(Reason::Crashed),
    });
    save(&spec.cache, &check);
    Some(check)
}

fn fresh(
    spec: &EncoderCheckSpec,
    runner: &Arc<dyn ProbeRunner>,
    deadline: Duration,
    key: String,
    previous: Option<&EncoderCheck>,
) -> EncoderCheck {
    let probes = runner.probes();
    let marker = marker_path(&spec.cache);
    let mut attempts = Vec::new();
    let mut verdicts = Vec::new();
    for probe in &probes {
        let outcome = if previous.is_some_and(|p| p.crashed_before(probe)) {
            Err((
                Reason::Crashed,
                "skipped, its check crashed with this driver".to_owned(),
            ))
        } else {
            write_marker(&marker, probe);
            let outcome = run_with_deadline(runner, &spec.cfg, probe, deadline);
            std::fs::remove_file(&marker).ok();
            outcome
        };
        let passed = outcome.is_ok();
        let reason = outcome.as_ref().err().map(|(reason, _)| *reason);
        verdicts.push(Verdict {
            probe: probe.clone(),
            reason,
        });
        let attempt = Attempt {
            kind: probe.kind,
            adapter: probe.adapter.clone(),
            vendor_id: probe.vendor_id,
            reason: reason.unwrap_or(Reason::Failed),
            result: outcome.map_err(|(_, error)| error),
        };
        log_attempt(&attempt);
        attempts.push(attempt);
        if passed {
            break;
        }
    }
    keep_untried_crashes(&mut verdicts, &probes, previous);
    let mut check = EncoderCheck::from_attempts(key, &attempts);
    check.verdicts = verdicts;
    check.noticed = previous.is_some_and(|p| p.key == check.key && p.noticed);
    match check.outcome {
        CheckOutcome::Failed => tracing::error!(detail = %check.detail, "no video encoder works"),
        _ => tracing::info!(encoder = %check.encoder, "encoder check done"),
    }
    save(&spec.cache, &check);
    check
}

fn keep_untried_crashes(
    verdicts: &mut Vec<Verdict>,
    probes: &[Probe],
    previous: Option<&EncoderCheck>,
) {
    let Some(previous) = previous else { return };
    for old in &previous.verdicts {
        let current = probes.iter().any(|p| p.same_target(&old.probe));
        let tried = verdicts.iter().any(|v| v.probe.same_target(&old.probe));
        if old.reason == Some(Reason::Crashed) && current && !tried {
            verdicts.push(old.clone());
        }
    }
}

fn log_attempt(attempt: &Attempt) {
    match &attempt.result {
        Ok(detail) => {
            tracing::info!(encoder = %attempt.label(), adapter = %attempt.adapter, %detail, "encoder check passed")
        }
        Err(error) => {
            tracing::warn!(encoder = %attempt.label(), adapter = %attempt.adapter, reason = ?attempt.reason, %error, "encoder check failed")
        }
    }
}

fn write_marker(path: &Path, probe: &Probe) {
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| {
            let json = serde_json::to_string(probe).map_err(std::io::Error::other)?;
            std::fs::write(path, json)
        });
    if let Err(err) = written {
        tracing::warn!(%err, "cannot write the encoder check marker");
    }
}

fn load(path: &Path) -> Option<EncoderCheck> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(check) => Some(check),
        Err(err) => {
            tracing::warn!(%err, path = %path.display(), "encoder check cache unreadable");
            None
        }
    }
}

fn save(path: &Path, check: &EncoderCheck) {
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| {
            let json = serde_json::to_string_pretty(check).map_err(std::io::Error::other)?;
            std::fs::write(path, json)
        });
    if let Err(err) = written {
        tracing::warn!(%err, path = %path.display(), "cannot save the encoder check");
    }
}

#[cfg(test)]
mod tests;
