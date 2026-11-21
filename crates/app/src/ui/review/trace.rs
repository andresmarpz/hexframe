use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;
use std::time::Instant;

use rune::{El, PlayerState};

use super::ReviewPlayer;

/// Player calls and player changes kept for a stall report.
const KEEP: usize = 200;
/// Playback that stands still this long, or a seek this slow, is a stall.
const STALL: f64 = 1.0;

thread_local! {
    static TRACE: RefCell<VecDeque<(Instant, String)>> = const { RefCell::new(VecDeque::new()) };
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

    fn seek_fast(&self, secs: f64) {
        self.call(format_args!("seek fast {secs:.3}"));
        self.0.seek_fast(secs);
    }

    fn seeking(&self) -> bool {
        self.0.seeking()
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

/// Watches one recording's player for stalls: playback that should move and doesn't, and seeks
/// that don't land. Each stall writes one report to the log.
#[derive(Debug, Default)]
pub(crate) struct Watch {
    seen: Option<(PlayerState, bool)>,
    /// The time while playing, and since when it has stood there.
    standing: Option<(f64, f64)>,
    /// Since when the player waits for a seek's picture, and the time it was asked for.
    seeking: Option<(f64, f64)>,
    /// A stall was reported and hasn't ended.
    stalled: Option<f64>,
    pub last_report: Option<String>,
    pub reports: usize,
}

impl Watch {
    pub(crate) fn check(
        &mut self,
        clock: f64,
        player: &dyn ReviewPlayer,
        scrubbing: bool,
        file: &Path,
    ) {
        let (state, seeking, time) = (player.state(), player.seeking(), player.time());
        if self.seen.as_ref() != Some(&(state.clone(), seeking)) {
            let waits = if seeking {
                ", waiting for a picture"
            } else {
                ""
            };
            note(format!("player {state:?}{waits} at {time:.3}"));
            self.seen = Some((state, seeking));
        }
        self.seeking = match self.seeking {
            Some(since) if seeking => Some(since),
            _ if seeking => Some((clock, time)),
            _ => None,
        };
        let moving = player.is_playing() && !seeking && !scrubbing;
        self.standing = match self.standing {
            Some((at, since)) if moving && at == time => Some((at, since)),
            _ if moving => Some((time, clock)),
            _ => None,
        };
        let stall = match (self.seeking, self.standing) {
            (Some((since, to)), _) if clock - since > STALL => {
                Some(("a seek waits for its picture", to, since))
            }
            (_, Some((at, since))) if clock - since > STALL => {
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
