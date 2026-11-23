use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::Path;
use std::time::Instant;

use rune::{El, PlayerState};

use super::ReviewPlayer;

/// Player calls and player changes kept for a stall report.
const KEEP: usize = 200;
/// Playback that stands still this long, or a seek this slow, is a stall.
const STALL: f64 = 1.0;
/// Playback is measured over this much of the frame clock.
const PACE_WINDOW: f64 = 0.5;
/// Playback faster than the rate times this, over a window, is a fast run.
const PACE_LIMIT: f64 = 1.5;
/// A step of the time this big in one frame is a jump (a seek, a loop), not a fast run.
const JUMP: f64 = 0.25;

thread_local! {
    static TRACE: RefCell<VecDeque<(Instant, String)>> = const { RefCell::new(VecDeque::new()) };
    static RATE: Cell<f32> = const { Cell::new(1.0) };
}

fn note(what: String) {
    TRACE.with_borrow_mut(|trace| {
        if trace.len() == KEEP {
            trace.pop_front();
        }
        trace.push_back((Instant::now(), what));
    });
}

fn recent() -> String {
    let now = Instant::now();
    TRACE.with_borrow(|trace| {
        trace
            .iter()
            .map(|(at, what)| {
                let ago = now.saturating_duration_since(*at).as_secs_f64();
                format!("  -{ago:.3} s  {what}\n")
            })
            .collect()
    })
}

/// A player whose calls go to the trace, each with the time and state it had.
#[derive(Debug)]
pub(crate) struct Traced(Box<dyn ReviewPlayer>);

pub(crate) fn traced(path: &Path, player: Box<dyn ReviewPlayer>) -> Box<dyn ReviewPlayer> {
    note(format!("open {}", path.display()));
    RATE.set(1.0);
    Box::new(Traced(player))
}

impl Traced {
    fn call(&self, what: std::fmt::Arguments) {
        let p = &self.0;
        note(format!("{what} at {:.3} ({:?})", p.time(), p.state()));
    }
}

impl ReviewPlayer for Traced {
    fn play(&self) {
        self.call(format_args!("play"));
        self.0.play();
    }

    fn pause(&self) {
        self.call(format_args!("pause"));
        self.0.pause();
    }

    fn seek(&self, secs: f64) {
        self.call(format_args!("seek {secs:.3}"));
        self.0.seek(secs);
    }

    fn time(&self) -> f64 {
        self.0.time()
    }

    fn duration(&self) -> Option<f64> {
        self.0.duration()
    }

    fn state(&self) -> PlayerState {
        self.0.state()
    }

    fn is_playing(&self) -> bool {
        self.0.is_playing()
    }

    fn set_rate(&self, rate: f32) {
        self.call(format_args!("rate {rate}"));
        RATE.set(rate);
        self.0.set_rate(rate);
    }

    fn set_muted(&self, muted: bool) {
        self.call(format_args!("muted {muted}"));
        self.0.set_muted(muted);
    }

    fn set_volume(&self, volume: f32) {
        self.call(format_args!("volume {volume:.2}"));
        self.0.set_volume(volume);
    }

    fn show(&self, frame: El) -> El {
        self.0.show(frame)
    }
}

/// Watches one recording's player for stalls: playback that should move and doesn't. Each stall writes one report to the log.
#[derive(Debug, Default)]
pub(crate) struct Watch {
    seen: Option<PlayerState>,
    /// The time while playing, and since when it has stood there.
    standing: Option<(f64, f64)>,
    /// A stall was reported and hasn't ended.
    stalled: Option<f64>,
    pub last_report: Option<String>,
    pub reports: usize,
    /// The frame clock and the time where the current measure of the pace began, and the last
    /// time seen.
    pace: Option<(f64, f64, f64)>,
    /// A fast run was reported and the pace has not been normal since.
    running_fast: bool,
    pub last_fast_report: Option<String>,
    pub fast_reports: usize,
}

impl Watch {
    pub(crate) fn check(
        &mut self,
        clock: f64,
        player: &dyn ReviewPlayer,
        scrubbing: bool,
        file: &Path,
    ) {
        let (state, time) = (player.state(), player.time());
        if self.seen.as_ref() != Some(&state) {
            note(format!("player {state:?} at {time:.3}"));
            self.seen = Some(state);
        }
        let moving = player.is_playing() && !scrubbing;
        self.standing = match self.standing {
            Some((at, since)) if moving && at == time => Some((at, since)),
            _ if moving => Some((time, clock)),
            _ => None,
        };
        self.measure_pace(clock, time, moving, file);
        let stall = match self.standing {
            Some((at, since)) if clock - since > STALL => {
                Some(("playing, but the time stands still", at, since))
            }
            _ => None,
        };
        match (stall, self.stalled) {
            (Some((what, at, since)), None) => {
                self.stalled = Some(since);
                let report = format!(
                    "playback stall: {what} at {at:.3} for {:.1} s in {}\nrecent player calls and changes:\n{}",
                    clock - since,
                    file.display(),
                    recent()
                );
                tracing::warn!("{report}");
                self.last_report = Some(report);
                self.reports += 1;
            }
            (None, Some(since)) => {
                self.stalled = None;
                note(format!("stall over after {:.1} s", clock - since));
                tracing::warn!(
                    "playback stall over after {:.1} s, at {time:.3}",
                    clock - since
                );
            }
            _ => {}
        }
    }
}

impl Watch {
    fn measure_pace(&mut self, clock: f64, time: f64, moving: bool, file: &Path) {
        let Some((began, from, last)) = self.pace.filter(|_| moving) else {
            self.pace = moving.then_some((clock, time, time));
            return;
        };
        if !(0.0..=JUMP).contains(&(time - last)) {
            self.pace = Some((clock, time, time));
            return;
        }
        self.pace = Some((began, from, time));
        let span = clock - began;
        if span < PACE_WINDOW {
            return;
        }
        let rate = f64::from(RATE.get());
        let advanced = time - from;
        let fast = advanced > rate * span * PACE_LIMIT;
        if fast && !self.running_fast {
            let report = format!(
                "playback ran fast: {advanced:.2} s of clip in {span:.2} s at rate {rate} in {}\nrecent player calls and changes:\n{}",
                file.display(),
                recent()
            );
            tracing::warn!("{report}");
            self.last_fast_report = Some(report);
            self.fast_reports += 1;
        }
        self.running_fast = fast;
        self.pace = Some((clock, time, time));
    }
}
