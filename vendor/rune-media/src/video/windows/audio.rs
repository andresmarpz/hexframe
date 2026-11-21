//! Sound for a clip: a WASAPI shared-mode stream on the default output device, fed by the
//! decoder (audio clips) or by a reader of its own (`ahead`, video), and timed by the player's
//! clock. Each time the device asks for more, it writes the samples for the moment they'll be
//! heard, from the device's own position, padding with silence or skipping ahead when the clock
//! jumped (a start, a seek, a stall). While it plays on, the sound is the master: where the
//! device clock drifts from the player's, the player's clock moves to the sound, so the
//! pictures follow what is heard and the sound never skips.

use super::ahead::{self, Read};
use super::machine::Anchor;
use super::mf::Sound;
use endpoint::Endpoint;
use rune_core::time::Instant;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;
use windows::Win32::Media::MediaFoundation::IMFSample;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

mod endpoint;
#[cfg(feature = "qa")]
pub(super) mod fake;

/// The device's buffer: long enough to ride out a busy decoder, short enough to react.
const BUFFER: Duration = Duration::from_millis(100);
/// Sound decoded ahead of what's heard; the reader waits beyond it.
const AHEAD: f64 = 1.0;
/// How far written sound may be from the clock before it jumps back in line: a start, a seek, a
/// stall, a clock the UI set again.
const SLOP: f64 = 0.04;
/// How far the sound may run from the clock, on average, before the clock moves to the sound:
/// a quarter of a picture at 60 fps, so one move never crosses a picture's boundary.
const NUDGE: f64 = 0.004;
/// Moves this soon after a start correct the start's estimate of the device's latency too.
const SETTLE: Duration = Duration::from_secs(1);
/// A stopped stream's unplayed sound is read again from this far before the clock stopped.
const REREAD: f64 = 0.1;
/// Each write's weight in that average.
const SMOOTH: f64 = 0.1;
/// The device's position is trusted only this far past the estimate from its buffer.
const POSITION_SLACK: f64 = 0.5;

/// The playback clock the UI sets and the sound follows: none while paused, stalled or ended.
#[derive(Default)]
pub(super) struct Clock {
    anchor: Mutex<Option<Anchor>>,
    changed: Condvar,
}

impl Clock {
    pub(super) fn set(&self, anchor: Option<Anchor>) {
        *self.anchor.lock().unwrap() = anchor;
        self.changed.notify_all();
    }
    pub(super) fn get(&self) -> Option<Anchor> {
        *self.anchor.lock().unwrap()
    }
    /// Moves the clock `by` seconds to where the sound plays, unless it was set again since
    /// `anchor` was read.
    fn nudge(&self, anchor: Anchor, by: f64) {
        let mut current = self.anchor.lock().unwrap();
        if *current == Some(anchor) {
            *current = Some(Anchor { pos: anchor.pos + by, ..anchor });
        }
    }
    /// Sleeps until the clock runs at 1x (or patience passes) while it doesn't: sound plays
    /// only at normal speed.
    fn wait_while_stopped(&self, patience: Duration) {
        let anchor = self.anchor.lock().unwrap();
        if anchor.is_none_or(|a| (a.rate - 1.0).abs() >= 1e-6) {
            let _ = self.changed.wait_timeout(anchor, patience);
        }
    }
}

struct Chunk {
    pts: f64,
    samples: Vec<f32>,
}

#[derive(Default)]
pub(super) struct Feed {
    chunks: VecDeque<Chunk>,
    /// Interleaved samples buffered.
    samples: usize,
    gain: f32,
    pub stop: bool,
    /// Media time of the next sample the device gets, once it's writing in line with the clock.
    cursor: Option<f64>,
    /// For the reader: an unwrapped time to read from next. Without a reader, nobody reads it.
    pub seek: Option<f64>,
    pub looping: bool,
}

impl Feed {
    /// Forgets queued sound.
    pub(super) fn clear(&mut self) {
        self.chunks.clear();
        self.samples = 0;
        self.cursor = None;
    }
}

/// The sound between whoever decodes it and the device; `room` wakes the reader when the device
/// took some, or something changed.
#[derive(Default)]
pub(super) struct Queue {
    feed: Mutex<Feed>,
    room: Condvar,
}

impl Queue {
    pub(super) fn lock(&self) -> MutexGuard<'_, Feed> {
        self.feed.lock().unwrap()
    }
    pub(super) fn wait<'a>(&self, feed: MutexGuard<'a, Feed>, patience: Duration) -> MutexGuard<'a, Feed> {
        self.room.wait_timeout(feed, patience).unwrap().0
    }
    /// Enough is queued; whoever decodes may wait.
    pub(super) fn full(&self, feed: &Feed, sound: Sound) -> bool {
        feed.samples as f64 >= AHEAD * f64::from(sound.rate) * f64::from(sound.channels)
    }
}

/// Queues a decoded float sample at `pts` seconds; sound before `skip_before` (a seek's
/// pre-roll) is dropped.
pub(super) fn push_sample(queue: &Queue, sound: Sound, sample: &IMFSample, pts: f64, skip_before: f64) {
    let Some(samples) = samples(sample) else { return };
    let channels = usize::from(sound.channels);
    let end = pts + (samples.len() / channels) as f64 / f64::from(sound.rate);
    if end <= skip_before {
        return;
    }
    let mut feed = queue.lock();
    feed.samples += samples.len();
    feed.chunks.push_back(Chunk { pts, samples });
}

/// A clip's sound output; dropping it stops the stream and its reader.
pub(super) struct Output {
    queue: Arc<Queue>,
    clock: Arc<Clock>,
    sound: Sound,
    /// A reader of its own feeds it (`read`); else the decoder pushes.
    reads: bool,
}

impl Output {
    /// Opens the stream; with `read`, a reader of its own reads the sound ahead.
    pub(super) fn open(sound: Sound, clock: Arc<Clock>, read: Option<Read>) -> Result<Self, String> {
        let queue = Arc::new(Queue::default());
        queue.lock().gain = 1.0;
        let (thread_queue, thread_clock) = (queue.clone(), clock.clone());
        let (ready, started) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("rune-video-sound".into())
            .spawn(move || {
                // SAFETY: COM for this thread, matched below.
                let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
                match endpoint(sound) {
                    Ok(device) => {
                        let _ = ready.send(Ok(()));
                        run(&device, &thread_queue, &thread_clock, sound);
                    }
                    Err(e) => {
                        let _ = ready.send(Err(e));
                    }
                }
                if com {
                    // SAFETY: matches the initialisation above.
                    unsafe { CoUninitialize() };
                }
            })
            .map_err(|e| e.to_string())?;
        started.recv().map_err(|_| "the sound thread stopped".to_string())??;
        let output = Self { queue, clock, sound, reads: read.is_some() };
        if let Some(read) = read {
            ahead::start(read, output.queue.clone(), sound)?;
        }
        Ok(output)
    }

    /// Queues a decoded float sample at `pts` seconds, from the decoder.
    pub(super) fn push(&self, sample: &IMFSample, pts: f64, skip_before: f64, _generation: u32) {
        push_sample(&self.queue, self.sound, sample, pts, skip_before);
    }

    /// Forgets queued sound; a reader reads again from `to` (unwrapped seconds).
    pub(super) fn seek(&self, to: f64) {
        let mut feed = self.queue.lock();
        feed.clear();
        if self.reads {
            feed.seek = Some(to);
        }
        drop(feed);
        self.queue.room.notify_all();
    }

    pub(super) fn set_gain(&self, gain: f32) {
        self.queue.lock().gain = gain.clamp(0.0, 1.0);
    }

    pub(super) fn set_looping(&self, looping: bool) {
        self.queue.lock().looping = looping;
    }

    /// Enough is queued from the decoder; it may wait. A reader of its own keeps it full.
    pub(super) fn full(&self) -> bool {
        !self.reads && self.queue.full(&self.queue.lock(), self.sound)
    }

    /// Nothing plays: paused, stalled or ended.
    pub(super) fn idle(&self) -> bool {
        self.clock.get().is_none()
    }

    /// The decoder reached the end; what's queued still plays out.
    pub(super) fn end(&self, _generation: u32) {}
}

impl Drop for Output {
    fn drop(&mut self) {
        self.queue.lock().stop = true;
        self.clock.changed.notify_all();
        self.queue.room.notify_all();
    }
}

/// A decoded float sample's interleaved values.
fn samples(sample: &IMFSample) -> Option<Vec<f32>> {
    // SAFETY: the buffer is locked for the copy and unlocked after it.
    unsafe {
        let buffer = sample.ConvertToContiguousBuffer().ok()?;
        let (mut base, mut len) = (std::ptr::null_mut(), 0u32);
        buffer.Lock(&mut base, None, Some(&mut len)).ok()?;
        let values = std::slice::from_raw_parts(base.cast::<f32>(), len as usize / 4).to_vec();
        let _ = buffer.Unlock();
        Some(values)
    }
}

/// The stream the sound plays on: the default output device, or the test device with `qa`.
#[cfg(not(feature = "qa"))]
fn endpoint(sound: Sound) -> Result<endpoint::Wasapi, String> {
    endpoint::Wasapi::new(sound)
}

/// The stream the sound plays on: the default output device, or the test device with `qa`.
#[cfg(feature = "qa")]
fn endpoint(sound: Sound) -> Result<fake::Fake, String> {
    Ok(fake::Fake::new(sound))
}

/// When the next sample written will be heard. Running, from the device's position: what was
/// written and not yet played is still to come. Else from the buffer, the stream's latency and
/// `bias`, the error earlier starts found in that estimate.
fn heard(device: &impl Endpoint, now: Instant, written: u64, padding: u32, rate: f64, running: bool, bias: f64) -> Instant {
    let buffered = (device.latency().as_secs_f64() + f64::from(padding) / rate + bias).max(0.0);
    let from_buffer = now + Duration::from_secs_f64(buffered);
    let Some((played, age)) = running.then(|| device.played()).flatten() else { return from_buffer };
    let wait = written as f64 / rate - played - age;
    if (0.0..=buffered + POSITION_SLACK).contains(&wait) { now + Duration::from_secs_f64(wait) } else { from_buffer }
}

fn run(device: &impl Endpoint, queue: &Queue, clock: &Clock, sound: Sound) {
    let channels = usize::from(sound.channels);
    let rate = f64::from(sound.rate);
    let mut started = false;
    // Frames written since the stream started, and how far the sound runs from the clock.
    let mut written = 0u64;
    let mut drift = 0.0;
    // When the stream last started, the clock it last played to, and the start estimate's error.
    let mut since: Option<Instant> = None;
    let mut last: Option<Anchor> = None;
    let mut bias = 0.0;
    loop {
        if !started {
            // Stopped: sleep until the clock runs, not on the device's period.
            clock.wait_while_stopped(Duration::from_secs(1));
        }
        let woke = started && device.wait(Duration::from_millis(20));
        if queue.lock().stop {
            break;
        }
        let anchor = clock.get().filter(|a| (a.rate - 1.0).abs() < 1e-6);
        let Some(anchor) = anchor else {
            if started {
                device.halt();
                started = false;
                written = 0;
                let mut feed = queue.lock();
                // The device's unplayed sound is gone: a reader reads it again, unless a
                // seek already cleared the queue.
                if let (Some(_), Some(last)) = (feed.cursor.take(), last) {
                    feed.seek = Some(last.position(Instant::now()) - REREAD);
                    drop(feed);
                    queue.room.notify_all();
                }
            }
            drift = 0.0;
            continue;
        };
        last = Some(anchor);
        if started && !woke {
            continue;
        }
        let padding = device.padding().unwrap_or(device.frames());
        let free = device.frames().saturating_sub(padding);
        if free == 0 {
            continue;
        }
        let Some(mut out) = device.lease(free) else { continue };
        let at = heard(device, Instant::now(), written, padding, rate, started, bias);
        let off = fill(&mut queue.lock(), &mut out, channels, rate, anchor.position(at));
        drop(out);
        device.release(free);
        written += u64::from(free);
        if !started {
            started = device.start();
            since = Some(Instant::now());
        }
        queue.room.notify_all();
        match off {
            Some(off) => {
                drift += (off - drift) * SMOOTH;
                if drift.abs() > NUDGE {
                    clock.nudge(anchor, drift);
                    if since.is_some_and(|t| t.elapsed() < SETTLE) {
                        bias -= drift;
                    }
                    drift = 0.0;
                }
            }
            None => drift = 0.0,
        }
    }
}

/// Writes `out` with the sound from media time `want` on. Within `SLOP` of the last write's
/// end, it goes on from there and returns how far that runs from `want`; further off, it jumps
/// to `want` and returns None.
fn fill(feed: &mut Feed, out: &mut [f32], channels: usize, rate: f64, want: f64) -> Option<f64> {
    let (mut cursor, off) = match feed.cursor {
        Some(c) if (c - want).abs() < SLOP => (c, Some(c - want)),
        _ => (want, None),
    };
    let frames = out.len() / channels;
    let mut written = 0;
    while written < frames {
        while let Some(c) = feed.chunks.front() {
            let end = c.pts + (c.samples.len() / channels) as f64 / rate;
            if end > cursor + 0.5 / rate {
                break;
            }
            feed.samples -= c.samples.len();
            feed.chunks.pop_front();
        }
        let Some(chunk) = feed.chunks.front() else {
            out[written * channels..].fill(0.0);
            cursor += (frames - written) as f64 / rate;
            break;
        };
        let start = ((cursor - chunk.pts) * rate).round() as i64;
        let n = if start < 0 {
            let n = ((-start) as usize).min(frames - written);
            out[written * channels..(written + n) * channels].fill(0.0);
            n
        } else {
            let start = start as usize;
            let n = (chunk.samples.len() / channels - start).min(frames - written);
            let gain = feed.gain;
            for (o, s) in out[written * channels..(written + n) * channels].iter_mut().zip(&chunk.samples[start * channels..]) {
                *o = s * gain;
            }
            n
        };
        written += n;
        cursor += n as f64 / rate;
    }
    feed.cursor = Some(cursor);
    off
}

#[cfg(test)]
mod tests;
