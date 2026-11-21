use super::ReviewState;

/// A drag asks for a preview picture at most this often.
const PREVIEW_EVERY: f64 = 0.1;
/// A pointer held still this long gets the exact picture before the release.
const REST: f64 = 0.25;

/// A drag on the seek bar or a marker. The thumb and the time follow the pointer at once; the
/// player gets one keyframe preview at a time and one exact seek where the drag stops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Scrub {
    pub target: f64,
    /// The clip played when the drag began: it plays again on release.
    resume: bool,
    /// Playback stopped for the drag.
    paused: bool,
    /// The last preview asked for: its time and when.
    preview: Option<(f64, f64)>,
    /// When the target last moved.
    moved: f64,
    /// The exact seek to the target was asked for.
    exact: bool,
}

impl ReviewState {
    pub(crate) fn scrubbing(&self) -> bool {
        self.scrub.is_some()
    }

    pub(crate) fn scrub_to(&mut self, secs: f64) {
        let target = secs.clamp(0.0, self.duration().max(0.0));
        let clock = self.clock;
        match &mut self.scrub {
            Some(scrub) if scrub.target != target => {
                scrub.target = target;
                scrub.moved = clock;
                scrub.exact = false;
            }
            Some(_) => {}
            None if self.player.is_some() => {
                self.scrub = Some(Scrub {
                    target,
                    resume: self.is_playing(),
                    paused: false,
                    preview: None,
                    moved: clock,
                    exact: false,
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
        if scrub.exact || player.seeking() {
            return;
        }
        let target = scrub.target;
        if clock - scrub.moved >= REST {
            scrub.exact = true;
            self.seek(target);
            return;
        }
        let due = scrub
            .preview
            .is_none_or(|(at, when)| at != target && clock - when >= PREVIEW_EVERY);
        if due {
            scrub.preview = Some((target, clock));
            player.seek_fast(target);
        }
    }

    pub(crate) fn scrub_end(&mut self) {
        let Some(scrub) = self.scrub.take() else {
            return;
        };
        if !scrub.exact {
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
