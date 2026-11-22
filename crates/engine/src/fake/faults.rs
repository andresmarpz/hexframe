use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailAt {
    After(Duration),
    AfterPackets(u32),
}

#[derive(Debug, Clone, Default)]
pub struct Faults {
    pub fail: Option<(FailAt, String)>,
    pub start_error: Option<String>,
    pub start_block: Duration,
    pub stop_block: Duration,
    pub audio_delay_secs: u32,
    pub audio_stop: Option<(u32, String)>,
    pub final_stats: bool,
}

#[derive(Debug, Default)]
pub struct FakeProbe {
    live: AtomicUsize,
    peak: AtomicUsize,
    max_send_wait_nanos: AtomicU64,
    packets: AtomicU64,
}

impl FakeProbe {
    pub fn live_sessions(&self) -> usize {
        self.live.load(Ordering::SeqCst)
    }

    pub fn peak_live_sessions(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    pub fn max_send_wait(&self) -> Duration {
        Duration::from_nanos(self.max_send_wait_nanos.load(Ordering::SeqCst))
    }

    pub fn packets_sent(&self) -> u64 {
        self.packets.load(Ordering::SeqCst)
    }

    pub(super) fn record_packet(&self) {
        self.packets.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn record_send_wait(&self, wait: Duration) {
        let nanos = u64::try_from(wait.as_nanos()).unwrap_or(u64::MAX);
        self.max_send_wait_nanos.fetch_max(nanos, Ordering::SeqCst);
    }

    pub(super) fn session_started(self: &Arc<Self>) -> LiveGuard {
        let live = self.live.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(live, Ordering::SeqCst);
        LiveGuard(self.clone())
    }
}

pub(super) struct LiveGuard(Arc<FakeProbe>);

impl Drop for LiveGuard {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, Ordering::SeqCst);
    }
}
