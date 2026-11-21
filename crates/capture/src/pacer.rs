use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now};

const SPIN_MARGIN: Hns = 15_000;
const MAX_LATE_FRAMES: i128 = 2;
const REPORT_EVERY: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub(crate) struct Pacer {
    t0: Hns,
    fps: u32,
    tick: u64,
}

impl Pacer {
    pub(crate) fn new(t0: Hns, fps: u32) -> Self {
        Self {
            t0,
            fps: fps.max(1),
            tick: 0,
        }
    }

    pub(crate) fn advance(&mut self) {
        self.tick += 1;
    }

    pub(crate) fn deadline(&self) -> Hns {
        self.t0 + self.offset(self.tick)
    }

    pub(crate) fn tick(&self) -> u64 {
        self.tick
    }

    pub(crate) fn pts(&self) -> Hns {
        self.offset(self.tick)
    }

    pub(crate) fn frame_hns(&self) -> Hns {
        HNS_PER_SEC / Hns::from(self.fps)
    }

    pub(crate) fn resync(&mut self, now: Hns) -> bool {
        if self.tick == 0 {
            return false;
        }
        let late = i128::from(now - self.deadline());
        if late <= MAX_LATE_FRAMES * i128::from(self.frame_hns()) {
            return false;
        }
        let elapsed = i128::from(now - self.t0);
        let tick = elapsed * i128::from(self.fps) / i128::from(HNS_PER_SEC);
        self.tick = u64::try_from(tick).unwrap_or(self.tick).max(self.tick);
        true
    }

    pub(crate) fn wait(&self, stop: &AtomicBool) -> bool {
        let deadline = self.deadline();
        loop {
            if stop.load(Ordering::Acquire) {
                return false;
            }
            let remaining = deadline - mono_now();
            if remaining <= 0 {
                return true;
            }
            if remaining > SPIN_MARGIN {
                let hns = u64::try_from(remaining - SPIN_MARGIN).unwrap_or(0);
                std::thread::sleep(Duration::from_nanos(hns * 100));
            } else {
                std::thread::yield_now();
            }
        }
    }

    fn offset(&self, tick: u64) -> Hns {
        (i128::from(tick) * i128::from(HNS_PER_SEC) / i128::from(self.fps)) as Hns
    }
}

#[derive(Debug)]
pub(crate) struct Stats {
    window_start: Instant,
    seq_mark: u64,
    encoded_mark: u64,
    pub(crate) dropped: u64,
    pub(crate) busy: u64,
    pub(crate) skips: u64,
    pub(crate) audio: u64,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self {
            window_start: Instant::now(),
            seq_mark: 0,
            encoded_mark: 0,
            dropped: 0,
            busy: 0,
            skips: 0,
            audio: 0,
        }
    }

    pub(crate) fn report_if_due(&mut self, seq: u64, encoded: u64) {
        if self.window_start.elapsed() < REPORT_EVERY {
            return;
        }
        tracing::info!(
            captured = seq.saturating_sub(self.seq_mark),
            encoded = encoded - self.encoded_mark,
            dropped_ticks = self.dropped,
            busy_slots = self.busy,
            skips = self.skips,
            audio_packets = self.audio,
            "capture stats for the last 10 s"
        );
        *self = Self {
            seq_mark: seq,
            encoded_mark: encoded,
            ..Self::new()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadlines_follow_exact_rational_ticks() {
        let mut pacer = Pacer::new(1_000, 60);
        for _ in 0..3 {
            pacer.advance();
        }
        assert_eq!(pacer.deadline(), 1_000 + 500_000);
        assert_eq!(pacer.pts(), 500_000);
        assert_eq!(pacer.frame_hns(), 166_666);
    }

    #[test]
    fn first_tick_never_resyncs() {
        let mut pacer = Pacer::new(0, 60);
        assert!(!pacer.resync(10 * HNS_PER_SEC));
        assert_eq!(pacer.tick(), 0);
        pacer.advance();
        assert!(pacer.resync(10 * HNS_PER_SEC));
    }

    #[test]
    fn resync_skips_to_real_time_only_when_far_behind() {
        let mut pacer = Pacer::new(0, 60);
        pacer.advance();
        assert!(!pacer.resync(2 * pacer.frame_hns()));
        assert!(pacer.resync(HNS_PER_SEC));
        assert_eq!(pacer.pts(), HNS_PER_SEC);
    }
}
