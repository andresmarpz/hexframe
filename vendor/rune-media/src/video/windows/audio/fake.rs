//! A render device for tests (`qa`): nothing reaches a speaker. It takes sound as a shared-mode
//! WASAPI stream does, on a 10 ms period at the clip's rate from a buffer the real one's size,
//! and keeps each stretch it played with the moment it reached the listener. Its position lags
//! what it took by `DELAY` while it reports `LATENCY`, so the start's estimate is off and the
//! sound's own timing (position to heard time, nudges, bias) has to correct it.
//!
//! It doesn't cover a real device's latency, position or period, nor a device change.

use super::super::mf::Sound;
use super::BUFFER;
use super::endpoint::Endpoint;
use rune_core::time::Instant;
use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

const PERIOD: Duration = Duration::from_millis(10);
const LATENCY: Duration = Duration::from_millis(10);
const DELAY: Duration = Duration::from_millis(30);

/// A stretch of sound the device played: `at` is when its first frame was heard.
#[derive(Clone, Debug)]
pub(crate) struct Heard {
    pub at: Instant,
    pub rate: f64,
    pub channels: usize,
    pub samples: Vec<f32>,
}

static HEARD: Mutex<Vec<Heard>> = Mutex::new(Vec::new());

/// What every test device played since the last call.
pub(crate) fn take() -> Vec<Heard> {
    std::mem::take(&mut *HEARD.lock().unwrap())
}

pub(crate) struct Fake {
    rate: f64,
    channels: usize,
    frames: u32,
    scratch: RefCell<Vec<f32>>,
    state: RefCell<State>,
}

struct State {
    /// While started: when the device takes its next frame.
    next: Option<Instant>,
    /// Frames taken since the stream started.
    taken: u64,
    /// Interleaved samples written and not taken yet.
    queued: VecDeque<f32>,
    /// The device's last period.
    tick: Instant,
}

impl Fake {
    pub(super) fn new(sound: Sound) -> Self {
        let rate = f64::from(sound.rate);
        let state = State { next: None, taken: 0, queued: VecDeque::new(), tick: Instant::now() };
        Self { rate, channels: usize::from(sound.channels).max(1), frames: (rate * BUFFER.as_secs_f64()).ceil() as u32, scratch: RefCell::default(), state: RefCell::new(state) }
    }

    /// Takes what the device played up to now. Out of sound, it waits, and goes on from the next
    /// write.
    fn advance(&self, s: &mut State) {
        let Some(next) = s.next else { return };
        let now = Instant::now();
        let due = (now.saturating_duration_since(next).as_secs_f64() * self.rate) as usize;
        let n = due.min(s.queued.len() / self.channels);
        if n > 0 {
            let samples: Vec<f32> = s.queued.drain(..n * self.channels).collect();
            HEARD.lock().unwrap().push(Heard { at: next + DELAY, rate: self.rate, channels: self.channels, samples });
            s.taken += n as u64;
        }
        s.next = Some(if n < due { now } else { next + Duration::from_secs_f64(n as f64 / self.rate) });
    }
}

impl Endpoint for Fake {
    type Lease<'a> = RefMut<'a, [f32]>;

    fn frames(&self) -> u32 {
        self.frames
    }

    fn latency(&self) -> Duration {
        LATENCY
    }

    fn padding(&self) -> Option<u32> {
        let mut s = self.state.borrow_mut();
        self.advance(&mut s);
        Some((s.queued.len() / self.channels) as u32)
    }

    fn lease(&self, frames: u32) -> Option<RefMut<'_, [f32]>> {
        let mut scratch = self.scratch.borrow_mut();
        scratch.clear();
        scratch.resize(frames as usize * self.channels, 0.0);
        Some(RefMut::map(scratch, |v| v.as_mut_slice()))
    }

    fn release(&self, frames: u32) {
        let mut s = self.state.borrow_mut();
        self.advance(&mut s);
        let scratch = self.scratch.borrow();
        s.queued.extend(&scratch[..(frames as usize * self.channels).min(scratch.len())]);
    }

    fn start(&self) -> bool {
        let mut s = self.state.borrow_mut();
        let now = Instant::now();
        s.next = Some(now);
        s.tick = now;
        true
    }

    fn halt(&self) {
        let mut s = self.state.borrow_mut();
        self.advance(&mut s);
        s.next = None;
        s.taken = 0;
        s.queued.clear();
    }

    fn wait(&self, patience: Duration) -> bool {
        let (tick, started) = {
            let mut s = self.state.borrow_mut();
            s.tick = (s.tick + PERIOD).max(Instant::now());
            (s.tick, s.next.is_some())
        };
        let left = tick.saturating_duration_since(Instant::now());
        std::thread::sleep(left.min(patience));
        started && left <= patience
    }

    fn played(&self) -> Option<(f64, f64)> {
        let mut s = self.state.borrow_mut();
        self.advance(&mut s);
        Some(((s.taken as f64 / self.rate - DELAY.as_secs_f64()).max(0.0), 0.0))
    }
}

#[cfg(test)]
mod tests;
