use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use greyscreen_core::health::CaptureStats;
use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now};

const SPIN_MARGIN: Hns = 15_000;
const MAX_LATE_FRAMES: i128 = 2;
const REPORT_EVERY: Duration = Duration::from_secs(10);
const SNAPSHOT_EVERY: Duration = Duration::from_secs(1);

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

#[derive(Debug, Clone, Copy, Default)]
struct Mark {
    captured: u64,
    encoded: u64,
    dropped: u64,
    busy: u64,
    resyncs: u64,
    audio: u64,
}

#[derive(Debug)]
pub(crate) struct Stats {
    pub(crate) c: CaptureStats,
    pub(crate) busy: u64,
    log_start: Instant,
    log_mark: Mark,
    snapshot_start: Instant,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self {
            c: CaptureStats::default(),
            busy: 0,
            log_start: Instant::now(),
            log_mark: Mark::default(),
            snapshot_start: Instant::now(),
        }
    }

    pub(crate) fn step_done(&mut self, started: Instant) {
        let us = u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX);
        self.c.window_step_max_us = self.c.window_step_max_us.max(us);
    }

    pub(crate) fn report_if_due(&mut self, encoded: u64) {
        if self.log_start.elapsed() < REPORT_EVERY {
            return;
        }
        let mark = self.log_mark;
        tracing::info!(
            captured = self.c.captured.saturating_sub(mark.captured),
            encoded = encoded.saturating_sub(mark.encoded),
            dropped_ticks = self.c.dropped.saturating_sub(mark.dropped),
            busy_slots = self.busy.saturating_sub(mark.busy),
            skips = self.c.resyncs.saturating_sub(mark.resyncs),
            audio_packets = self.c.audio_packets.saturating_sub(mark.audio),
            "capture stats for the last 10 s"
        );
        self.log_mark = Mark {
            captured: self.c.captured,
            encoded,
            dropped: self.c.dropped,
            busy: self.busy,
            resyncs: self.c.resyncs,
            audio: self.c.audio_packets,
        };
        self.log_start = Instant::now();
    }

    pub(crate) fn snapshot_due(&mut self) -> bool {
        if self.snapshot_start.elapsed() < SNAPSHOT_EVERY {
            return false;
        }
        self.snapshot_start = Instant::now();
        true
    }

    pub(crate) fn take_snapshot(&mut self, at: Hns) -> Box<CaptureStats> {
        self.c.at = at;
        let snapshot = Box::new(self.c.clone());
        self.c.window_step_max_us = 0;
        snapshot
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
