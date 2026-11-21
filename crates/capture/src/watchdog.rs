use greyscreen_core::time::{HNS_PER_SEC, Hns};

const STALL_LIMIT: Hns = 5 * HNS_PER_SEC;

#[derive(Debug, Clone)]
pub(crate) struct Watchdog {
    last_progress: Hns,
    encoded: u64,
}

impl Watchdog {
    pub(crate) fn new(now: Hns) -> Self {
        Self {
            last_progress: now,
            encoded: 0,
        }
    }

    pub(crate) fn stalled(&mut self, now: Hns, encoded: u64, submitting: bool) -> bool {
        if encoded != self.encoded || !submitting {
            self.encoded = encoded;
            self.last_progress = now;
            return false;
        }
        now - self.last_progress >= STALL_LIMIT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalls_after_five_seconds_without_output_while_submitting() {
        let mut dog = Watchdog::new(0);
        assert!(!dog.stalled(4 * HNS_PER_SEC, 0, true));
        assert!(dog.stalled(5 * HNS_PER_SEC, 0, true));
    }

    #[test]
    fn output_resets_the_window() {
        let mut dog = Watchdog::new(0);
        assert!(!dog.stalled(4 * HNS_PER_SEC, 1, true));
        assert!(!dog.stalled(8 * HNS_PER_SEC, 1, true));
        assert!(dog.stalled(9 * HNS_PER_SEC, 1, true));
    }

    #[test]
    fn idle_capture_never_stalls() {
        let mut dog = Watchdog::new(0);
        assert!(!dog.stalled(30 * HNS_PER_SEC, 0, false));
        assert!(!dog.stalled(34 * HNS_PER_SEC, 0, true));
        assert!(dog.stalled(35 * HNS_PER_SEC, 0, true));
    }
}
