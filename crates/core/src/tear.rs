use std::time::Duration;

pub const RING_SLOTS: usize = 3;
pub const THUMB_MAX_WIDTH: u32 = 64;
pub const COPY_WAIT_BUDGET: Duration = Duration::from_millis(12);
pub const COPY_WAIT_SLOW: Duration = Duration::from_millis(4);

const CELL_DELTA: u8 = 40;
const MIN_CELL_SHARE: usize = 64;
const MIN_CELLS: usize = 4;
const NEIGHBOUR_RATIO: usize = 4;
const SPIN_FOR: Duration = Duration::from_micros(500);
const SLEEP_STEP: Duration = Duration::from_micros(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    Held,
    Published(usize),
    Dropped(usize),
}

#[derive(Debug, Clone, Default)]
pub struct HoldBack {
    thumbs: [Vec<u8>; RING_SLOTS],
    published: Option<usize>,
    pending: Option<usize>,
}

impl HoldBack {
    pub fn write_slot(&self) -> usize {
        (0..RING_SLOTS)
            .find(|&slot| Some(slot) != self.published && Some(slot) != self.pending)
            .unwrap_or(0)
    }

    pub fn thumb_mut(&mut self, slot: usize) -> &mut Vec<u8> {
        &mut self.thumbs[slot % RING_SLOTS]
    }

    pub fn published(&self) -> Option<usize> {
        self.published
    }

    pub fn pending(&self) -> Option<usize> {
        self.pending
    }

    pub fn arrive(&mut self, slot: usize) -> Arrival {
        let Some(candidate) = self.pending.replace(slot) else {
            return Arrival::Held;
        };
        let torn = self.published.is_some_and(|prev| {
            let thumbs = &self.thumbs;
            let diff = |a: usize, b: usize| changed_cells(&thumbs[a], &thumbs[b]);
            match (
                diff(prev, candidate),
                diff(candidate, slot),
                diff(prev, slot),
            ) {
                (Some(a), Some(b), Some(c)) => is_torn(a, b, c, thumbs[slot].len()),
                _ => false,
            }
        });
        if torn {
            Arrival::Dropped(candidate)
        } else {
            self.published = Some(candidate);
            Arrival::Published(candidate)
        }
    }

    pub fn publish_now(&mut self, slot: usize) {
        self.published = Some(slot);
        self.pending = None;
    }

    pub fn promote(&mut self) -> Option<usize> {
        let slot = self.pending.take()?;
        self.published = Some(slot);
        Some(slot)
    }

    pub fn reset(&mut self) {
        self.published = None;
        self.pending = None;
    }
}

pub fn changed_cells(a: &[u8], b: &[u8]) -> Option<usize> {
    if a.is_empty() || a.len() != b.len() {
        return None;
    }
    Some(
        a.iter()
            .zip(b)
            .filter(|(x, y)| x.abs_diff(**y) >= CELL_DELTA)
            .count(),
    )
}

pub fn is_torn(prev_cur: usize, cur_next: usize, prev_next: usize, cells: usize) -> bool {
    let floor = (cells / MIN_CELL_SHARE).max(MIN_CELLS);
    let jump = prev_cur.min(cur_next);
    jump >= floor && jump >= prev_next.saturating_mul(NEIGHBOUR_RATIO)
}

pub fn thumb_level(size: (u32, u32)) -> u32 {
    let mut level = 0;
    while (size.0 >> level) > THUMB_MAX_WIDTH && (size.1 >> (level + 1)) >= 1 {
        level += 1;
    }
    level
}

pub fn thumb_size(size: (u32, u32), level: u32) -> (u32, u32) {
    ((size.0 >> level).max(1), (size.1 >> level).max(1))
}

pub fn luma(r: u8, g: u8, b: u8) -> u8 {
    ((54 * u32::from(r) + 183 * u32::from(g) + 19 * u32::from(b)) >> 8) as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    Done(Duration),
    TimedOut(Duration),
}

impl Wait {
    pub fn waited(self) -> Duration {
        match self {
            Self::Done(waited) | Self::TimedOut(waited) => waited,
        }
    }
}

pub fn wait_bounded<E>(
    budget: Duration,
    mut elapsed: impl FnMut() -> Duration,
    mut signaled: impl FnMut() -> Result<bool, E>,
    mut pause: impl FnMut(Duration),
) -> Result<Wait, E> {
    loop {
        let waited = elapsed();
        if signaled()? {
            return Ok(Wait::Done(waited));
        }
        if waited >= budget {
            return Ok(Wait::TimedOut(waited));
        }
        pause(if waited < SPIN_FOR {
            Duration::ZERO
        } else {
            SLEEP_STEP
        });
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CopyWaits {
    pub slow: u64,
    pub timeouts: u64,
    pub max_us: u32,
}

impl CopyWaits {
    pub fn record(&mut self, wait: Wait) {
        let waited = wait.waited();
        let us = u32::try_from(waited.as_micros()).unwrap_or(u32::MAX);
        self.max_us = self.max_us.max(us);
        match wait {
            Wait::TimedOut(_) => self.timeouts += 1,
            Wait::Done(_) if waited >= COPY_WAIT_SLOW => self.slow += 1,
            Wait::Done(_) => {}
        }
    }
}

#[cfg(test)]
mod tests;
