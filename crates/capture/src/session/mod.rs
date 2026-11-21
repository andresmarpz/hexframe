mod audio;
mod emit;
mod facts;
mod matching;
mod pick;
mod setup;
mod video;

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use greyscreen_core::encoder::Attempt;
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource};

const SETUP_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Default)]
pub(crate) struct Flags {
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) keyframe: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsMediaSource;

impl WindowsMediaSource {
    pub fn new() -> Self {
        Self
    }
}

impl MediaSource for WindowsMediaSource {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        if cfg.fps == 0 {
            return Err("capture fps must be above zero".into());
        }
        let flags = Flags::default();
        let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
        let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(0);
        let thread_flags = flags.clone();
        let thread = std::thread::Builder::new()
            .name("greyscreen-video".into())
            .spawn(move || {
                let _done = done_tx;
                thread_main(target, cfg, out, thread_flags, ready_tx);
            })
            .map_err(|e| format!("could not spawn the video thread: {e}"))?;
        let session = Session {
            thread: Some(thread),
            flags,
            done: done_rx,
        };
        match ready_rx.recv_timeout(SETUP_TIMEOUT) {
            Ok(Ok(())) => Ok(Box::new(session)),
            Ok(Err(message)) => Err(message),
            Err(_) => Err("capture setup timed out".into()),
        }
    }

    fn encoder_fingerprint(&self) -> Option<String> {
        Some(crate::selftest::fingerprint())
    }

    fn check_encoders(&self, cfg: &CaptureConfig, every: bool) -> Vec<Attempt> {
        crate::selftest::check(cfg, every)
    }
}

struct Session {
    thread: Option<JoinHandle<()>>,
    flags: Flags,
    done: Receiver<()>,
}

impl MediaSession for Session {
    fn request_keyframe(&self) {
        self.flags.keyframe.store(true, Ordering::Release);
    }

    fn stop(mut self: Box<Self>) {
        self.flags.stop.store(true, Ordering::Release);
        match self.done.recv_timeout(STOP_TIMEOUT) {
            Err(RecvTimeoutError::Timeout) => {
                tracing::warn!("video thread did not stop within {STOP_TIMEOUT:?}");
            }
            _ => {
                if let Some(thread) = self.thread.take()
                    && thread.join().is_err()
                {
                    tracing::warn!("video thread panicked");
                }
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.flags.stop.store(true, Ordering::Release);
    }
}

struct Ready {
    tx: Sender<Result<(), String>>,
    sent: AtomicBool,
}

impl Ready {
    fn report(&self, result: Result<(), String>) -> bool {
        let first = !self.sent.swap(true, Ordering::AcqRel);
        if first {
            self.tx.send(result).ok();
        }
        first
    }
}

fn thread_main(
    target: CaptureTarget,
    cfg: CaptureConfig,
    out: Sender<MediaEvent>,
    flags: Flags,
    ready_tx: Sender<Result<(), String>>,
) {
    let ready = Ready {
        tx: ready_tx,
        sent: AtomicBool::new(false),
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        match setup::build(&target, &cfg, out.clone(), flags) {
            Ok(video) => {
                ready.report(Ok(()));
                video.run();
            }
            Err(message) => {
                ready.report(Err(message));
            }
        }
    }));
    if let Err(payload) = result {
        let message = format!("capture thread panicked: {}", panic_text(payload.as_ref()));
        tracing::error!("{message}");
        if ready.report(Err(message.clone())) {
            return;
        }
        out.send(MediaEvent::Failed(message)).ok();
    }
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}
