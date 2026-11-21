use std::cell::{Cell, RefCell};
use std::path::Path;

use rune::{El, PlayerState};

use super::{Call, record};
use crate::ui::review::ReviewPlayer;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Media {
    pub duration: f64,
    pub pictures_end: f64,
    pub seek_latency: f64,
    pub layered: bool,
}

impl Media {
    pub(crate) fn new(duration: f64) -> Self {
        Media {
            duration,
            pictures_end: duration,
            seek_latency: 0.0,
            layered: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Run {
    Paused,
    Playing,
    Ended,
}

#[derive(Debug, Clone)]
struct Sim {
    media: Media,
    run: Run,
    rate: f64,
    position: f64,
    reported: f64,
    picture: f64,
    seek: Option<(f64, f64)>,
    muted: bool,
    volume: f32,
}

thread_local! {
    static SIM: RefCell<Option<Sim>> = const { RefCell::new(None) };
    static HEARD: Cell<f64> = const { Cell::new(0.0) };
}

pub(crate) fn simulate(media: Media) {
    SIM.set(Some(Sim {
        media,
        run: Run::Paused,
        rate: 1.0,
        position: 0.0,
        reported: 0.0,
        picture: 0.0,
        seek: None,
        muted: true,
        volume: 1.0,
    }));
}

fn with<T>(f: impl FnOnce(&mut Sim) -> T) -> T {
    SIM.with_borrow_mut(|sim| f(sim.as_mut().expect("simulate first")))
}

pub(crate) fn advance(secs: f64) {
    with(|sim| sim.advance(secs));
}

pub(crate) fn heard() -> f64 {
    HEARD.get()
}

pub(crate) fn reset_heard() {
    HEARD.set(0.0);
}

pub(crate) fn picture() -> f64 {
    with(|sim| sim.picture)
}

pub(crate) fn landed() -> bool {
    with(|sim| sim.seek.is_none())
}

impl Sim {
    fn show(&mut self, at: f64) {
        self.picture = at;
        self.reported = at;
    }

    fn advance(&mut self, secs: f64) {
        if let Some((to, left)) = self.seek {
            let left = left - secs;
            if left > 0.0 {
                self.seek = Some((to, left));
            } else {
                let at = to.min(self.media.pictures_end);
                self.seek = None;
                self.position = at;
                self.show(at);
            }
            return;
        }
        if self.run != Run::Playing {
            return;
        }
        self.position += secs * self.rate;
        if self.position >= self.media.duration {
            self.position = self.media.duration;
            self.run = Run::Ended;
            self.picture = self.media.pictures_end;
            self.reported = self.media.duration;
            return;
        }
        if !self.media.layered {
            self.show(self.position.min(self.media.pictures_end));
        }
        self.play_sound(secs);
    }

    fn play_sound(&self, secs: f64) {
        if !self.muted && self.volume > 0.0 && self.rate == 1.0 {
            HEARD.set(HEARD.get() + secs);
        }
    }

    fn pause(&mut self) {
        if self.run != Run::Playing {
            return;
        }
        self.run = Run::Paused;
        if self.seek.is_none() {
            self.show(self.position.min(self.media.pictures_end));
        }
    }

    fn seek(&mut self, secs: f64) {
        let to = secs.clamp(0.0, self.media.duration);
        self.reported = to;
        self.seek = Some((to, self.media.seek_latency));
        if self.run == Run::Ended {
            self.run = Run::Paused;
        }
        if self.media.seek_latency <= 0.0 {
            self.advance(0.0);
        }
    }

    fn play(&mut self) {
        match self.run {
            Run::Playing => return,
            Run::Ended => self.seek(0.0),
            Run::Paused => {}
        }
        self.run = Run::Playing;
        if self.seek.is_none() {
            self.show(self.position.min(self.media.pictures_end));
        }
    }

    fn time(&self) -> f64 {
        if self.run == Run::Playing && self.seek.is_none() {
            self.position
        } else {
            self.reported
        }
    }
}

#[derive(Debug)]
pub(crate) struct SimPlayer;

pub(crate) fn factory(path: &Path) -> Box<dyn ReviewPlayer> {
    record(Call::Open(path.to_owned()));
    let media = with(|sim| sim.media);
    simulate(media);
    with(|sim| sim.muted = false);
    Box::new(SimPlayer)
}

impl ReviewPlayer for SimPlayer {
    fn play(&self) {
        record(Call::Play);
        with(Sim::play);
    }

    fn pause(&self) {
        record(Call::Pause);
        with(Sim::pause);
    }

    fn seek(&self, secs: f64) {
        record(Call::Seek(secs));
        with(|sim| sim.seek(secs));
    }

    fn time(&self) -> f64 {
        with(|sim| sim.time())
    }

    fn duration(&self) -> Option<f64> {
        Some(with(|sim| sim.media.duration))
    }

    fn state(&self) -> PlayerState {
        match with(|sim| sim.run) {
            Run::Paused => PlayerState::Ready,
            Run::Playing => PlayerState::Playing,
            Run::Ended => PlayerState::Ended,
        }
    }

    fn is_playing(&self) -> bool {
        with(|sim| sim.run == Run::Playing)
    }

    fn set_rate(&self, rate: f32) {
        record(Call::Rate(rate));
        with(|sim| sim.rate = f64::from(rate));
    }

    fn set_muted(&self, muted: bool) {
        record(Call::Muted(muted));
        with(|sim| sim.muted = muted);
    }

    fn set_volume(&self, volume: f32) {
        record(Call::Volume(volume));
        with(|sim| sim.volume = volume);
    }

    fn show(&self, frame: El) -> El {
        frame
    }
}
