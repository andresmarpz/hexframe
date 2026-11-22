use std::time::Duration;

pub const RING_SLOTS: usize = 4;
pub const THUMB_MAX_WIDTH: u32 = 128;
pub const COPY_WAIT_BUDGET: Duration = Duration::from_millis(12);
pub const COPY_WAIT_SLOW: Duration = Duration::from_millis(4);

const CELL_DELTA: u8 = 40;
const CONTENT_DELTA: u8 = 4;
const UNCHANGED_CELL_SHARE: usize = 300;
const UNCHANGED_MAX_CELLS: usize = 16;
const HISTORY_SLOTS: usize = 6;
const STALE_SLOTS: usize = 2;
const MAX_CONSECUTIVE_REWINDS: u8 = 4;
const REWIND_MOVE_MIN_X100: u32 = 22;
const REWIND_MATCH_MAX_X100: u32 = 15;
const REWIND_RATIO: u32 = 3;
const REWIND_CELL_DELTA: u8 = 2;
const REWIND_AREA_SHARE: usize = 32;
const MIN_CELL_SHARE: usize = 64;
const MIN_CELLS: usize = 4;
const NEIGHBOUR_RATIO: usize = 4;
const TILE: usize = 8;
const TILE_SHARE: usize = 4;
const HUD_ZONES: [[usize; 4]; 3] = [
    [750, 650, 1001, 1001],
    [240, 780, 760, 1001],
    [850, 0, 1001, 100],
];
const SPIN_FOR: Duration = Duration::from_micros(500);
const SLEEP_STEP: Duration = Duration::from_micros(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    Held,
    Published(usize),
    Dropped(usize),
    Merged { content_changed: bool },
    Rewound,
}

#[derive(Debug, Clone, Default)]
pub struct HoldBack {
    thumbs: [Vec<u8>; RING_SLOTS],
    widths: [usize; RING_SLOTS],
    published: Option<usize>,
    pending: Option<usize>,
    reading: Option<usize>,
    history: [Vec<u8>; HISTORY_SLOTS],
    history_next: usize,
    stale: [Vec<u8>; STALE_SLOTS],
    stale_next: usize,
    consecutive_rewinds: u8,
}

impl HoldBack {
    pub fn write_slot(&self) -> usize {
        (0..RING_SLOTS)
            .find(|&slot| ![self.published, self.pending, self.reading].contains(&Some(slot)))
            .unwrap_or(0)
    }

    pub fn start_reading(&mut self) -> Option<usize> {
        self.reading = self.published;
        self.reading
    }

    pub fn done_reading(&mut self) {
        self.reading = None;
    }

    pub fn reading(&self) -> Option<usize> {
        self.reading
    }

    pub fn store_thumb(&mut self, slot: usize, thumb: &mut Vec<u8>, width: usize) {
        std::mem::swap(&mut self.thumbs[slot % RING_SLOTS], thumb);
        self.widths[slot % RING_SLOTS] = width;
    }

    pub fn published(&self) -> Option<usize> {
        self.published
    }

    pub fn pending(&self) -> Option<usize> {
        self.pending
    }

    pub fn arrive(&mut self, slot: usize) -> Arrival {
        if self.is_rewind(slot) {
            return Arrival::Rewound;
        }
        if self.consecutive_rewinds >= MAX_CONSECUTIVE_REWINDS {
            self.clear_stale();
        }
        self.consecutive_rewinds = 0;

        let Some(candidate) = self.pending else {
            if self
                .published
                .is_some_and(|prev| is_same_content(&self.thumbs[prev], &self.thumbs[slot]))
            {
                self.published = Some(slot);
                return Arrival::Merged {
                    content_changed: false,
                };
            }
            self.pending = Some(slot);
            return Arrival::Held;
        };
        if is_same_content(&self.thumbs[candidate], &self.thumbs[slot]) {
            self.record_published();
            self.published = Some(slot);
            self.pending = None;
            return Arrival::Merged {
                content_changed: true,
            };
        }
        self.pending = Some(slot);
        let torn = self.published.is_some_and(|prev| {
            let width = self.widths[slot];
            let same_width = self.widths[prev] == width && self.widths[candidate] == width;
            is_torn_frame(
                [
                    &self.thumbs[prev],
                    &self.thumbs[candidate],
                    &self.thumbs[slot],
                ],
                if same_width { width } else { 0 },
            )
        });
        if torn {
            self.record_published();
            self.published = Some(slot);
            self.pending = None;
            Arrival::Dropped(candidate)
        } else {
            self.record_published();
            self.published = Some(candidate);
            Arrival::Published(candidate)
        }
    }

    fn is_rewind(&mut self, slot: usize) -> bool {
        if self.consecutive_rewinds >= MAX_CONSECUTIVE_REWINDS {
            return false;
        }
        let Some(newest) = self.pending.or(self.published) else {
            return false;
        };
        if let (Some(prev), Some(candidate)) = (self.published, self.pending) {
            let width = self.widths[slot];
            let same_width = self.widths[prev] == width && self.widths[candidate] == width;
            if is_torn_frame(
                [
                    &self.thumbs[prev],
                    &self.thumbs[candidate],
                    &self.thumbs[slot],
                ],
                if same_width { width } else { 0 },
            ) {
                return false;
            }
        }
        let Some((d_newest, moved)) = diff_stats(&self.thumbs[newest], &self.thumbs[slot]) else {
            return false;
        };
        if d_newest < REWIND_MOVE_MIN_X100 || moved < self.thumbs[slot].len() / REWIND_AREA_SHARE {
            return false;
        }
        let matches_prev_pub = match (self.published, self.pending) {
            (Some(prev), Some(_)) => mad_x100(&self.thumbs[prev], &self.thumbs[slot])
                .is_some_and(|d| is_rewind_match(d_newest, d)),
            _ => false,
        };
        let matches_old = matches_prev_pub
            || self.history.iter().any(|old| {
                mad_x100(old, &self.thumbs[slot]).is_some_and(|d| is_rewind_match(d_newest, d))
            })
            || self.stale.iter().any(|old| {
                mad_x100(old, &self.thumbs[slot]).is_some_and(|d| is_rewind_match(d_newest, d))
            });
        if !matches_old {
            return false;
        }
        self.remember_stale(slot);
        self.consecutive_rewinds = self.consecutive_rewinds.saturating_add(1);
        true
    }

    fn record_published(&mut self) {
        let Some(prev) = self.published else {
            return;
        };
        let thumb = &self.thumbs[prev];
        if thumb.is_empty() {
            return;
        }
        let dst = &mut self.history[self.history_next];
        dst.clear();
        dst.extend_from_slice(thumb);
        self.history_next = (self.history_next + 1) % HISTORY_SLOTS;
    }

    fn remember_stale(&mut self, slot: usize) {
        let thumb = &self.thumbs[slot];
        if thumb.is_empty()
            || self
                .stale
                .iter()
                .any(|s| mad_x100(s, thumb).is_some_and(|d| d <= REWIND_MATCH_MAX_X100))
        {
            return;
        }
        let dst = &mut self.stale[self.stale_next];
        dst.clear();
        dst.extend_from_slice(thumb);
        self.stale_next = (self.stale_next + 1) % STALE_SLOTS;
    }

    fn clear_stale(&mut self) {
        for s in &mut self.stale {
            s.clear();
        }
        self.stale_next = 0;
    }

    pub fn publish_now(&mut self, slot: usize) {
        self.record_published();
        self.published = Some(slot);
        self.pending = None;
        self.consecutive_rewinds = 0;
    }

    pub fn promote(&mut self) -> Option<usize> {
        let slot = self.pending.take()?;
        self.record_published();
        self.published = Some(slot);
        Some(slot)
    }

    pub fn reset(&mut self) {
        self.published = None;
        self.pending = None;
        self.reading = None;
        for h in &mut self.history {
            h.clear();
        }
        self.history_next = 0;
        self.clear_stale();
        self.consecutive_rewinds = 0;
    }
}

fn is_rewind_match(d_newest: u32, d_old: u32) -> bool {
    d_old <= REWIND_MATCH_MAX_X100 && d_newest >= d_old.saturating_mul(REWIND_RATIO)
}

fn diff_stats(a: &[u8], b: &[u8]) -> Option<(u32, usize)> {
    if a.is_empty() || a.len() != b.len() {
        return None;
    }
    let mut sum = 0u64;
    let mut moved = 0usize;
    for (&x, &y) in a.iter().zip(b) {
        let d = x.abs_diff(y);
        sum += u64::from(d);
        moved += usize::from(d >= REWIND_CELL_DELTA);
    }
    let mad = ((sum * 100) / a.len() as u64).min(u64::from(u32::MAX)) as u32;
    Some((mad, moved))
}

pub fn mad_x100(a: &[u8], b: &[u8]) -> Option<u32> {
    diff_stats(a, b).map(|(mad, _)| mad)
}

pub fn is_same_content(a: &[u8], b: &[u8]) -> bool {
    if a.is_empty() || a.len() != b.len() {
        return false;
    }
    let max_cells = (a.len() / UNCHANGED_CELL_SHARE).min(UNCHANGED_MAX_CELLS);
    let mut changed = 0usize;
    for (x, y) in a.iter().zip(b) {
        if x.abs_diff(*y) >= CONTENT_DELTA {
            changed += 1;
            if changed > max_cells {
                return false;
            }
        }
    }
    true
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

pub fn is_torn_frame([prev, cur, next]: [&[u8]; 3], width: usize) -> bool {
    let whole = match (
        changed_cells(prev, cur),
        changed_cells(cur, next),
        changed_cells(prev, next),
    ) {
        (Some(a), Some(b), Some(c)) => is_torn(a, b, c, next.len()),
        _ => return false,
    };
    whole || has_torn_tile([prev, cur, next], width)
}

pub fn has_torn_tile([prev, cur, next]: [&[u8]; 3], width: usize) -> bool {
    let len = cur.len();
    if width == 0
        || len == 0
        || !len.is_multiple_of(width)
        || prev.len() != len
        || next.len() != len
    {
        return false;
    }
    let height = len / width;
    let changed = |x: u8, y: u8| usize::from(x.abs_diff(y) >= CELL_DELTA);
    (0..height).step_by(TILE).any(|top| {
        let rows = top..(top + TILE).min(height);
        (0..width).step_by(TILE).any(|left| {
            let cols = left..(left + TILE).min(width);
            if !over_hud(&cols, &rows, width, height) {
                return false;
            }
            let (mut a, mut b, mut c) = (0, 0, 0);
            for y in rows.clone() {
                let row = y * width;
                for i in cols.clone().map(|x| row + x) {
                    a += changed(prev[i], cur[i]);
                    b += changed(cur[i], next[i]);
                    c += changed(prev[i], next[i]);
                }
            }
            is_torn_tile(a, b, c, rows.len() * cols.len())
        })
    })
}

fn over_hud(
    cols: &std::ops::Range<usize>,
    rows: &std::ops::Range<usize>,
    width: usize,
    height: usize,
) -> bool {
    let x = (cols.start + cols.end) * 500 / width;
    let y = (rows.start + rows.end) * 500 / height;
    HUD_ZONES
        .iter()
        .any(|[x0, y0, x1, y1]| (*x0..*x1).contains(&x) && (*y0..*y1).contains(&y))
}

fn is_torn_tile(prev_cur: usize, cur_next: usize, prev_next: usize, cells: usize) -> bool {
    let floor = (cells / TILE_SHARE).max(MIN_CELLS);
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
