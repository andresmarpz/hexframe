use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use greyscreen_core::frame_timing::Pacer;
use greyscreen_core::health::CaptureStats;
use greyscreen_core::time::{Hns, mono_now};

const SPIN_MARGIN: Hns = 15_000;
const REPORT_EVERY: Duration = Duration::from_secs(10);
const SNAPSHOT_EVERY: Duration = Duration::from_secs(1);

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
    first_submit: Option<Instant>,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self {
            c: CaptureStats::default(),
            busy: 0,
            log_start: Instant::now(),
            log_mark: Mark::default(),
            snapshot_start: Instant::now(),
            first_submit: None,
        }
    }

    pub(crate) fn note_wake(&mut self, started: Instant, spin_started: Option<Instant>, late: Hns) {
        let micros = |d: Duration| u64::try_from(d.as_micros()).unwrap_or(u64::MAX);
        let late_us = u32::try_from(late / 10).unwrap_or(u32::MAX);
        self.c.wake_lateness.0.record(late_us);
        let spin = spin_started.map_or(0, |at| micros(at.elapsed()));
        self.c.pacer_spin.add(spin, micros(started.elapsed()));
    }

    pub(crate) fn note_submit(&mut self) {
        self.first_submit.get_or_insert_with(Instant::now);
    }

    pub(crate) fn note_encoded(&mut self, encoded: u64) {
        if encoded == 0 || self.c.first_output_ms.is_some() {
            return;
        }
        if let Some(at) = self.first_submit {
            self.c.first_output_ms =
                Some(u32::try_from(at.elapsed().as_millis()).unwrap_or(u32::MAX));
        }
    }

    pub(crate) fn note_encoder_stall(&mut self, encoded: u64) {
        if encoded == 0 {
            self.c.encoder_stalls_early += 1;
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

pub(crate) fn wait_for(pacer: &Pacer, stop: &AtomicBool, stats: &mut Stats) -> bool {
    let started = Instant::now();
    let mut spin_started: Option<Instant> = None;
    loop {
        if stop.load(Ordering::Acquire) {
            return false;
        }
        let remaining = pacer.remaining(mono_now());
        if remaining <= 0 {
            stats.note_wake(started, spin_started, -remaining);
            return true;
        }
        if remaining > SPIN_MARGIN {
            let hns = u64::try_from(remaining - SPIN_MARGIN).unwrap_or(0);
            std::thread::sleep(Duration::from_nanos(hns * 100));
        } else {
            spin_started.get_or_insert_with(Instant::now);
            std::thread::yield_now();
        }
    }
}
