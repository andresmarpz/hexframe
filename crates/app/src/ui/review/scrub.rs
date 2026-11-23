use super::ReviewState;

/// A drag asks the player for the pointer's picture at most this often.
const SEEK_EVERY: f64 = 0.25;

/// A drag on the seek bar or a marker. The thumb and the time follow the pointer at once; the
/// player gets a seek every `SEEK_EVERY` and one more where the drag stops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Scrub {
    pub target: f64,
    /// The clip played when the drag began: it plays again on release.
    resume: bool,
    /// Playback stopped for the drag.
    paused: bool,
    /// The last seek asked for: its time and when.
    sought: Option<(f64, f64)>,
}

impl ReviewState {
    pub(crate) fn scrubbing(&self) -> bool {
        self.scrub.is_some()
    }

    pub(crate) fn scrub_to(&mut self, secs: f64) {
        let target = secs.clamp(0.0, self.duration().max(0.0));
        match &mut self.scrub {
            Some(scrub) => scrub.target = target,
            None if self.player.is_some() => {
                self.scrub = Some(Scrub {
                    target,
                    resume: self.is_playing(),
                    paused: false,
                    sought: None,
                });
            }
            None => {}
        }
    }

    pub(crate) fn scrub_tick(&mut self, clock: f64) {
        let (Some(player), Some(scrub)) = (&self.player, &mut self.scrub) else {
            return;
        };
        if !scrub.paused {
            if scrub.resume {
                player.pause();
            }
            scrub.paused = true;
        }
        let due = scrub
            .sought
            .is_none_or(|(at, when)| at != scrub.target && clock - when >= SEEK_EVERY);
        if due {
            scrub.sought = Some((scrub.target, clock));
            player.seek(scrub.target);
        }
    }

    pub(crate) fn scrub_end(&mut self) {
        let Some(scrub) = self.scrub.take() else {
            return;
        };
        if scrub.sought.is_none_or(|(at, _)| at != scrub.target) {
            self.seek(scrub.target);
        }
        if scrub.resume
            && scrub.paused
            && let Some(player) = &self.player
        {
            player.play();
        }
    }
}
