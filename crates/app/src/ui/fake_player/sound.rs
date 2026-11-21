#![allow(clippy::expect_used)]

use std::cell::{Cell, RefCell};

use rune::{El, PlayerState};

use super::{Call, record};
use crate::ui::review::ReviewPlayer;

const OPEN_SECS: f64 = 0.15;
pub(crate) const SYNC_SLACK: f64 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Run {
    Paused,
    Playing,
    Ended,
}

#[derive(Debug, Clone)]
struct SoundClip {
    duration: f64,
    opening: f64,
    deselected: bool,
    run: Run,
    position: f64,
    rate: f64,
    muted: bool,
    volume: f32,
}

thread_local! {
    static CLIP: RefCell<Option<SoundClip>> = const { RefCell::new(None) };
    static HEARD: Cell<f64> = const { Cell::new(0.0) };
    static DRIFT: Cell<f64> = const { Cell::new(0.0) };
}

pub(crate) fn heard() -> f64 {
    HEARD.get()
}

pub(crate) fn drift() -> f64 {
    DRIFT.get()
}

pub(crate) fn reset_heard() {
    HEARD.set(0.0);
    DRIFT.set(0.0);
}

pub(crate) fn hear(secs: f64) {
    HEARD.set(HEARD.get() + secs);
}

pub(crate) fn forget() {
    CLIP.set(None);
}

pub(crate) fn open(duration: f64) -> Box<dyn ReviewPlayer> {
    CLIP.set(Some(SoundClip {
        duration,
        opening: OPEN_SECS,
        deselected: false,
        run: Run::Paused,
        position: 0.0,
        rate: 1.0,
        muted: false,
        volume: 1.0,
    }));
    Box::new(SoundPlayer)
}

pub(crate) fn advance(secs: f64, picture: Option<f64>) {
    CLIP.with_borrow_mut(|clip| {
        if let Some(clip) = clip {
            clip.advance(secs, picture);
        }
    });
}

fn with<T>(f: impl FnOnce(&mut SoundClip) -> T) -> T {
    CLIP.with_borrow_mut(|clip| f(clip.as_mut().expect("open a sound clip first")))
}

impl SoundClip {
    fn advance(&mut self, secs: f64, picture: Option<f64>) {
        if self.opening > 0.0 {
            self.opening = (self.opening - secs).max(0.0);
            return;
        }
        if self.run != Run::Playing {
            return;
        }
        self.position += secs * self.rate;
        if self.position >= self.duration {
            self.position = self.duration;
            self.run = Run::Ended;
            return;
        }
        let audible = self.rate == 1.0 && !self.muted && self.volume > 0.0 && !self.deselected;
        if audible {
            hear(secs);
            if let Some(picture) = picture {
                DRIFT.set(DRIFT.get().max((self.position - picture).abs()));
            }
        }
    }

    fn edit(&mut self) {
        if self.opening > 0.0 {
            self.deselected = true;
        }
    }
}

#[derive(Debug)]
pub(crate) struct SoundPlayer;

impl ReviewPlayer for SoundPlayer {
    fn play(&self) {
        record(Call::SoundPlay);
        with(|clip| {
            clip.edit();
            if clip.run == Run::Ended {
                clip.position = 0.0;
            }
            clip.run = Run::Playing;
        });
    }

    fn pause(&self) {
        record(Call::SoundPause);
        with(|clip| {
            clip.edit();
            if clip.run == Run::Playing {
                clip.run = Run::Paused;
            }
        });
    }

    fn seek(&self, secs: f64) {
        record(Call::SoundSeek(secs));
        with(|clip| {
            clip.edit();
            clip.position = secs.clamp(0.0, clip.duration);
            if clip.run == Run::Ended {
                clip.run = Run::Paused;
            }
            if clip.opening <= 0.0 && !clip.muted {
                clip.deselected = false;
            }
        });
    }

    fn time(&self) -> f64 {
        with(|clip| clip.position)
    }

    fn duration(&self) -> Option<f64> {
        with(|clip| (clip.opening <= 0.0).then_some(clip.duration))
    }

    fn state(&self) -> PlayerState {
        with(|clip| match clip.run {
            _ if clip.opening > 0.0 => PlayerState::Loading,
            Run::Paused => PlayerState::Ready,
            Run::Playing => PlayerState::Playing,
            Run::Ended => PlayerState::Ended,
        })
    }

    fn is_playing(&self) -> bool {
        with(|clip| clip.run == Run::Playing)
    }

    fn set_rate(&self, rate: f32) {
        record(Call::SoundRate(rate));
        with(|clip| {
            clip.edit();
            clip.rate = f64::from(rate);
        });
    }

    fn set_muted(&self, muted: bool) {
        record(Call::SoundMuted(muted));
        with(|clip| {
            clip.edit();
            clip.deselected |= muted;
            clip.muted = muted;
        });
    }

    fn set_volume(&self, volume: f32) {
        record(Call::SoundVolume(volume));
        with(|clip| {
            clip.edit();
            clip.volume = volume;
        });
    }

    fn show(&self, frame: El) -> El {
        frame
    }
}
