use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use greyscreen_core::encoder::{Probe, Reason};
use greyscreen_core::media::{CaptureConfig, MediaSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    Passed(String),
    Failed(String),
    Crashed(String),
}

pub trait ProbeRunner: Send + Sync {
    fn probes(&self) -> Vec<Probe>;
    fn run(&self, cfg: &CaptureConfig, probe: &Probe) -> ProbeResult;
}

pub struct InProcessRunner {
    media: Arc<dyn MediaSource>,
}

impl std::fmt::Debug for InProcessRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessRunner").finish_non_exhaustive()
    }
}

impl InProcessRunner {
    pub fn new(media: Arc<dyn MediaSource>) -> Self {
        Self { media }
    }
}

impl ProbeRunner for InProcessRunner {
    fn probes(&self) -> Vec<Probe> {
        self.media.encoder_probes()
    }

    fn run(&self, cfg: &CaptureConfig, probe: &Probe) -> ProbeResult {
        match self.media.probe_encoder(cfg, probe) {
            Ok(detail) => ProbeResult::Passed(detail),
            Err(error) => ProbeResult::Failed(error),
        }
    }
}

pub(super) fn run_with_deadline(
    runner: &Arc<dyn ProbeRunner>,
    cfg: &CaptureConfig,
    probe: &Probe,
    deadline: Duration,
) -> Result<String, (Reason, String)> {
    let (tx, rx) = mpsc::channel();
    let (runner, cfg, probe) = (runner.clone(), cfg.clone(), probe.clone());
    let spawned = std::thread::Builder::new()
        .name("encoder-probe".into())
        .spawn(move || {
            tx.send(runner.run(&cfg, &probe)).ok();
        });
    if let Err(err) = spawned {
        return Err((Reason::Failed, format!("cannot start the probe: {err}")));
    }
    match rx.recv_timeout(deadline) {
        Ok(ProbeResult::Passed(detail)) => Ok(detail),
        Ok(ProbeResult::Failed(error)) => Err((Reason::Failed, error)),
        Ok(ProbeResult::Crashed(error)) => Err((Reason::Crashed, error)),
        Err(mpsc::RecvTimeoutError::Timeout) => Err((
            Reason::TimedOut,
            format!("no answer within {} s", deadline.as_secs_f32()),
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err((Reason::Crashed, "the probe ended without an answer".into()))
        }
    }
}
