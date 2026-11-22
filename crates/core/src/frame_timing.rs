use std::collections::VecDeque;

use crate::time::{HNS_PER_SEC, Hns};

const MAX_LATE_FRAMES: i128 = 2;
const STALL_LIMIT: Hns = 5 * HNS_PER_SEC;

#[derive(Debug, Clone)]
pub struct Pacer {
    t0: Hns,
    fps: u32,
    tick: u64,
}

impl Pacer {
    pub fn new(t0: Hns, fps: u32) -> Self {
        Self {
            t0,
            fps: fps.max(1),
            tick: 0,
        }
    }

    pub fn advance(&mut self) {
        self.tick += 1;
    }

    pub fn deadline(&self) -> Hns {
        self.t0 + self.offset(self.tick)
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn pts(&self) -> Hns {
        self.offset(self.tick)
    }

    pub fn frame_hns(&self) -> Hns {
        HNS_PER_SEC / Hns::from(self.fps)
    }

    pub fn resync(&mut self, now: Hns) -> bool {
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

    fn offset(&self, tick: u64) -> Hns {
        (i128::from(tick) * i128::from(HNS_PER_SEC) / i128::from(self.fps)) as Hns
    }
}

#[derive(Debug, Clone)]
pub struct Watchdog {
    last_progress: Hns,
    encoded: u64,
}

impl Watchdog {
    pub fn new(now: Hns) -> Self {
        Self {
            last_progress: now,
            encoded: 0,
        }
    }

    pub fn stalled(&mut self, now: Hns, encoded: u64, submitting: bool) -> bool {
        if encoded != self.encoded || !submitting {
            self.encoded = encoded;
            self.last_progress = now;
            return false;
        }
        now - self.last_progress >= STALL_LIMIT
    }
}

pub fn match_tick(
    in_flight: &mut VecDeque<(u64, Hns)>,
    output_pts: Hns,
    first_output: bool,
) -> Option<u64> {
    if output_pts == 0 && !first_output {
        return in_flight.pop_front().map(|(tick, _)| tick);
    }
    while in_flight.front().is_some_and(|&(_, pts)| pts < output_pts) {
        in_flight.pop_front();
    }
    match in_flight.front() {
        Some(&(tick, pts)) if pts == output_pts => {
            in_flight.pop_front();
            Some(tick)
        }
        _ => None,
    }
}

pub fn next_tick(
    in_flight: &mut VecDeque<(u64, Hns)>,
    last_tick: Option<u64>,
    output_pts: Hns,
    first_output: bool,
) -> u64 {
    let fallback = last_tick.map_or(0, |last| last + 1);
    let matched = match_tick(in_flight, output_pts, first_output);
    matched.map_or(fallback, |tick| tick.max(fallback))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepBack {
    pub previous: Hns,
    pub pts: Hns,
}

#[derive(Debug, Clone, Default)]
pub struct OrderGuard {
    last: Option<Hns>,
    step_backs: u64,
}

impl OrderGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&mut self, pts: Hns) -> Result<(), StepBack> {
        let previous = self.last.replace(pts);
        match previous {
            Some(previous) if pts <= previous => {
                self.step_backs += 1;
                Err(StepBack { previous, pts })
            }
            _ => Ok(()),
        }
    }

    pub fn step_backs(&self) -> u64 {
        self.step_backs
    }
}

pub fn count_skipped(in_flight: &VecDeque<(u64, Hns)>, output_pts: Hns, first_output: bool) -> u64 {
    if output_pts == 0 && !first_output {
        return 0;
    }
    in_flight
        .iter()
        .take_while(|&&(_, pts)| pts < output_pts)
        .count() as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pending {
    pub tick: u64,
    pub pts: Hns,
    pub slot: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SlotRelease {
    last: Option<Hns>,
    one_per_output: bool,
}

impl SlotRelease {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn one_per_output(&self) -> bool {
        self.one_per_output
    }

    pub fn on_output(&mut self, pending: &mut VecDeque<Pending>, output_pts: Hns) -> Vec<usize> {
        if self.last.is_some_and(|last| output_pts <= last) {
            self.one_per_output = true;
        }
        self.last = Some(output_pts);
        if self.one_per_output {
            return pending
                .pop_front()
                .map(|entry| entry.slot)
                .into_iter()
                .collect();
        }
        let mut slots = Vec::new();
        while pending.front().is_some_and(|entry| entry.pts <= output_pts) {
            slots.extend(pending.pop_front().map(|entry| entry.slot));
        }
        slots
    }

    pub fn flush(pending: &mut VecDeque<Pending>) -> Vec<usize> {
        pending.drain(..).map(|entry| entry.slot).collect()
    }
}

#[cfg(test)]
mod tests;
