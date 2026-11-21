use std::cell::Cell;

use rune::{El, PlayerState};

use super::review::ReviewPlayer;

const DRIFT_SECS: f64 = 0.15;
const STILL_SECS: f64 = 0.25;

#[derive(Debug, Clone, Copy)]
struct Wanted {
    volume: f32,
    muted: bool,
    rate: f32,
}

#[derive(Debug, Clone, Copy)]
struct Seen {
    time: f64,
    moved_at: Option<f64>,
}

#[derive(Debug)]
pub(crate) struct PairedSound {
    video: Box<dyn ReviewPlayer>,
    sound: Box<dyn ReviewPlayer>,
    wanted: Cell<Wanted>,
    seen: Cell<Seen>,
    gain: Cell<Option<f32>>,
}

impl PairedSound {
    pub(crate) fn new(video: Box<dyn ReviewPlayer>, sound: Box<dyn ReviewPlayer>) -> Self {
        video.set_muted(true);
        PairedSound {
            video,
            sound,
            wanted: Cell::new(Wanted {
                volume: 1.0,
                muted: false,
                rate: 1.0,
            }),
            seen: Cell::new(Seen {
                time: 0.0,
                moved_at: None,
            }),
            gain: Cell::new(None),
        }
    }

    fn sound_ready(&self) -> bool {
        !matches!(
            self.sound.state(),
            PlayerState::Loading | PlayerState::Failed(_)
        )
    }

    fn wanted_gain(&self) -> f32 {
        let wanted = self.wanted.get();
        if wanted.muted { 0.0 } else { wanted.volume }
    }

    fn apply_gain(&self) {
        let gain = self.wanted_gain();
        if self.sound_ready() && self.gain.get() != Some(gain) {
            self.sound.set_volume(gain);
            self.gain.set(Some(gain));
        }
    }

    fn hold(&self) {
        self.seen.set(Seen {
            time: self.video.time(),
            moved_at: None,
        });
        if self.sound_ready() && self.sound.is_playing() {
            self.sound.pause();
        }
    }

    fn edit(&self, change: impl FnOnce(&mut Wanted)) {
        let mut wanted = self.wanted.get();
        change(&mut wanted);
        self.wanted.set(wanted);
    }
}

impl ReviewPlayer for PairedSound {
    fn play(&self) {
        self.video.play();
    }

    fn pause(&self) {
        self.video.pause();
        self.hold();
    }

    fn seek(&self, secs: f64) {
        self.video.seek(secs);
        self.hold();
    }

    fn time(&self) -> f64 {
        self.video.time()
    }

    fn duration(&self) -> Option<f64> {
        self.video.duration()
    }

    fn state(&self) -> PlayerState {
        self.video.state()
    }

    fn is_playing(&self) -> bool {
        self.video.is_playing()
    }

    fn set_rate(&self, rate: f32) {
        self.video.set_rate(rate);
        self.edit(|wanted| wanted.rate = rate);
        if rate != 1.0 {
            self.hold();
        }
    }

    fn set_muted(&self, muted: bool) {
        self.edit(|wanted| wanted.muted = muted);
        self.apply_gain();
    }

    fn set_volume(&self, volume: f32) {
        self.edit(|wanted| wanted.volume = volume);
        self.apply_gain();
    }

    fn show(&self, frame: El) -> El {
        self.video.show(frame)
    }

    fn follow(&self, clock: f64) {
        let time = self.video.time();
        let mut seen = self.seen.get();
        if time != seen.time {
            seen = Seen {
                time,
                moved_at: Some(clock),
            };
            self.seen.set(seen);
        }
        if !self.sound_ready() {
            return;
        }
        self.apply_gain();
        let moving = seen.moved_at.is_some_and(|at| clock - at < STILL_SECS);
        let flowing = moving
            && self.video.is_playing()
            && self.wanted.get().rate == 1.0
            && self.wanted_gain() > 0.0;
        match (flowing, self.sound.is_playing()) {
            (true, false) => {
                self.sound.seek(time);
                self.sound.play();
            }
            (true, true) if (self.sound.time() - time).abs() > DRIFT_SECS => {
                self.sound.seek(time);
            }
            (false, true) => self.sound.pause(),
            _ => {}
        }
    }
}
