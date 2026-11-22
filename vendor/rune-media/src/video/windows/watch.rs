//! A few lines per seek or pause for the app's log, whether or not `RUNE_VIDEO_LOG` is set: phase
//! changes, seeks, restarts, long decoder waits and a picture that stands still while the clock
//! runs. Nothing here logs per picture.

use super::machine::{Anchor, Phase};
use rune_core::time::Instant;
use std::time::Duration;

/// Clock seconds with no new picture before the picture counts as frozen.
const FROZEN: f64 = 1.0;

const LATENESS_BUCKETS: usize = 200;
const LATENESS_BUCKET_US: u32 = 100;

/// Per-play summary of how late layer presents woke relative to their target refresh.
pub(super) struct PresentLateness {
    buckets: [u32; LATENESS_BUCKETS],
    count: u64,
    max_us: u32,
}

impl Default for PresentLateness {
    fn default() -> Self {
        Self { buckets: [0; LATENESS_BUCKETS], count: 0, max_us: 0 }
    }
}

impl PresentLateness {
    pub(super) fn record(&mut self, late: Duration) {
        let us = u32::try_from(late.as_micros()).unwrap_or(u32::MAX);
        let index = ((us / LATENESS_BUCKET_US) as usize).min(LATENESS_BUCKETS - 1);
        self.buckets[index] = self.buckets[index].saturating_add(1);
        self.count += 1;
        self.max_us = self.max_us.max(us);
    }

    pub(super) fn percentile_ms(&self, p: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let target = (self.count as f64 * p).ceil().max(1.0) as u64;
        let mut seen = 0u64;
        for (index, &n) in self.buckets.iter().enumerate() {
            seen += u64::from(n);
            if seen >= target {
                let upper = (index as u32 + 1) * LATENESS_BUCKET_US;
                return f64::from(upper.min(self.max_us.max(1))) / 1000.0;
            }
        }
        f64::from(self.max_us) / 1000.0
    }

    pub(super) fn flush(&mut self, reason: &str) {
        if self.count == 0 {
            return;
        }
        tracing::info!(
            presents = self.count,
            p50_ms = (self.percentile_ms(0.50) * 100.0).round() / 100.0,
            p95_ms = (self.percentile_ms(0.95) * 100.0).round() / 100.0,
            max_ms = (f64::from(self.max_us) / 10.0).round() / 100.0,
            reason,
            "video layer present lateness"
        );
        *self = Self::default();
    }
}

/// A phase with its time, for one log line.
pub(super) fn phase(p: Phase) -> String {
    let play = |play: bool| if play { ", then plays" } else { "" };
    match p {
        Phase::Idle { at } => format!("idle at {at:.3}"),
        Phase::Opening { at, play: p } => format!("opening at {at:.3}{}", play(p)),
        Phase::Buffering { at, play: p } => format!("buffering at {at:.3}{}", play(p)),
        Phase::Playing { clock } => format!("playing at {:.3} ({}x)", clock.position(Instant::now()), clock.rate),
        Phase::Paused { at } => format!("paused at {at:.3}"),
        Phase::Ended { at } => format!("ended at {at:.3}"),
        Phase::Failed => "failed".into(),
    }
}

/// Watches the newest picture shown (on the surface or the layer) against the running clock.
#[derive(Default)]
pub(super) struct Freeze {
    /// The newest picture, when it came and the clock's position then.
    last: Option<(Option<f64>, Instant, f64)>,
    /// A freeze was reported and hasn't ended.
    frozen: bool,
}

impl Freeze {
    /// One look: `clock` while playing, and the newest picture shown (None: none yet).
    pub(super) fn look(&mut self, clock: Option<Anchor>, picture: Option<f64>, layered: bool, now: Instant) {
        let Some(clock) = clock else {
            if let (true, Some((_, since, _))) = (self.frozen, self.last) {
                tracing::info!("picture frozen until playback stopped, {:.1} s", now.saturating_duration_since(since).as_secs_f64());
            }
            (self.last, self.frozen) = (None, false);
            return;
        };
        let pos = clock.position(now);
        match self.last {
            Some((shown, since, at)) if shown == picture => {
                let ran = pos - at;
                if !self.frozen && ran > FROZEN {
                    self.frozen = true;
                    let shown = shown.map_or("none".into(), |s| format!("{s:.3}"));
                    let on = if layered { "the layer" } else { "the surface" };
                    tracing::warn!(
                        "picture frozen: clock advanced {ran:.1} s since the last presented picture ({shown} on {on}, clock at {pos:.3}, {:.1} s ago)",
                        now.saturating_duration_since(since).as_secs_f64()
                    );
                }
            }
            last => {
                if let (true, Some((_, since, _))) = (self.frozen, last) {
                    let at = picture.map_or("none".into(), |p| format!("{p:.3}"));
                    tracing::info!("picture moves again at {at} after {:.1} s frozen", now.saturating_duration_since(since).as_secs_f64());
                }
                (self.last, self.frozen) = (Some((picture, now, pos)), false);
            }
        }
    }
}
