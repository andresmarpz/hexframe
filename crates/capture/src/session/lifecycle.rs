use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError};

const ABANDONED_WAIT: Duration = Duration::from_secs(5);

static LIVE: AtomicUsize = AtomicUsize::new(0);
static ABANDONED: Mutex<Option<Receiver<()>>> = Mutex::new(None);

pub(super) struct LiveGuard;

impl LiveGuard {
    pub(super) fn enter() -> Self {
        let live = LIVE.fetch_add(1, Ordering::AcqRel) + 1;
        if live > 1 {
            tracing::warn!(live, "more than one capture session is alive");
        }
        Self
    }
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg_attr(not(any(test, feature = "fault-injection")), allow(dead_code))]
pub fn live_sessions() -> usize {
    LIVE.load(Ordering::Acquire)
}

pub(super) fn abandon(done: Receiver<()>) {
    *ABANDONED.lock().unwrap_or_else(PoisonError::into_inner) = Some(done);
}

pub(super) fn wait_for_abandoned() -> Result<(), String> {
    let Some(done) = ABANDONED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
    else {
        return Ok(());
    };
    match done.recv_timeout(ABANDONED_WAIT) {
        Err(RecvTimeoutError::Timeout) => {
            abandon(done);
            Err("the previous capture attempt is still shutting down".into())
        }
        _ => Ok(()),
    }
}
