//! The real Windows player under test, with nothing a person at the PC would notice: Media
//! Foundation decodes on the GPU into the player's shared textures, Rune's surface pipeline draws
//! each picture into an offscreen texture that is read back, and the sound plays into the test
//! device (`audio::fake`). No window shows, nothing takes focus, nothing is heard.
//!
//! The clip's pictures carry their frame number as ten black or white bars, so each drawn picture
//! names itself and can be checked against `Player::time`.

use super::super::{Clock, LayerStatus, Player, PlayerState};
use super::audio::fake::Heard;
use super::machine::Anchor;
use crate::native_surface::SurfaceRenderer;
use crate::native_surface::gpu_tests::{Gpu, OFFSCREEN_FORMAT, draw, render};
use rune_core::time::Instant;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

mod tests;
mod window;

pub(super) const FPS: f64 = 60.0;
const BARS: u32 = 10;
const VIEW: (u32, u32) = (160, 90);
const REFRESH: Duration = Duration::from_nanos(16_666_667);

pub(super) fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("qa-bars-60fps.m4v")
}

/// The fixture's frame number for a clip time.
pub(super) fn frame_of(time: f64) -> u32 {
    (time * FPS).round() as u32
}

/// One frame of the window the test stands in for.
#[derive(Clone, Debug)]
pub(super) struct Shot {
    pub at: Instant,
    pub time: f64,
    /// The frame number of the picture the frame drew; None before the first.
    pub picture: Option<u32>,
    pub state: PlayerState,
}

pub(super) struct Rig {
    pub player: Player,
    renderer: SurfaceRenderer,
    wake: Arc<rune_core::live::Wake>,
    /// The player's clock after each frame, for placing what was heard.
    clocks: Vec<(Instant, Option<Anchor>)>,
    gpu: Gpu,
}

impl Rig {
    pub(super) fn open(path: &Path, sound: bool) -> Self {
        let gpu = Gpu::headless_wgpu();
        crate::native_surface::note_device(&gpu.device, true);
        crate::video::enable_playback();
        super::audio::fake::take();
        let player = Player::file(path);
        player.set_muted(!sound);
        let renderer = SurfaceRenderer::new(&gpu.device, OFFSCREEN_FORMAT);
        Self { player, renderer, wake: rune_core::live::Wake::new(Arc::new(|| {})), clocks: Vec::new(), gpu }
    }

    /// Pulls the clip for the next refresh and draws it offscreen, as a window's frame does.
    pub(super) fn frame(&mut self) -> Shot {
        let surface = self.player.surface().clone();
        surface.pull_video(Clock::Display(Instant::now()), &self.wake);
        let picture = surface.latest().map(|_| {
            let pixels = render(&self.gpu, &mut self.renderer, &[draw(&surface, [0.0, 0.0, VIEW.0 as f32, VIEW.1 as f32])], VIEW);
            bars(&pixels)
        });
        let at = Instant::now();
        self.clocks.push((at, self.clock()));
        Shot { at, time: self.player.time(), picture, state: self.player.state() }
    }

    /// Frames at the display's rate for `secs`, with `each` run after every frame.
    pub(super) fn run_with(&mut self, secs: f64, mut each: impl FnMut(&mut Self, &Shot)) -> Vec<Shot> {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        let mut shots = Vec::new();
        let mut next = Instant::now();
        while Instant::now() < end {
            let shot = self.frame();
            each(self, &shot);
            shots.push(shot);
            next += REFRESH;
            std::thread::sleep(next.saturating_duration_since(Instant::now()));
        }
        shots
    }

    pub(super) fn run(&mut self, secs: f64) -> Vec<Shot> {
        self.run_with(secs, |_, _| {})
    }

    /// Frames until `done` holds, or panics after `secs`.
    pub(super) fn until(&mut self, secs: f64, what: &str, mut done: impl FnMut(&mut Self, &Shot) -> bool) -> Shot {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        loop {
            let shot = self.frame();
            if done(self, &shot) {
                return shot;
            }
            assert!(Instant::now() < end, "{what}: not after {secs} s; last frame {shot:?}");
            std::thread::sleep(REFRESH);
        }
    }

    /// The clock the sound and the layer follow; None while nothing plays.
    pub(super) fn clock(&self) -> Option<Anchor> {
        self.player.shared.native.lock().unwrap().clock.get()
    }

    /// Whether pictures reach Rune on the GPU, and whether a hardware decoder made them.
    pub(super) fn decoding(&self) -> Option<(bool, bool)> {
        self.player.shared.native.lock().unwrap().facts.map(|f| (f.gpu, f.hardware))
    }

    /// Asks for the clip's layer at `size` (None: the window draws the pictures), as the window
    /// does each frame.
    pub(super) fn layer(&self, ask: Option<(u32, u32)>) -> LayerStatus {
        crate::video::layer(&self.player.shared, ask)
    }

    /// The window takes the pictures back from the layer.
    pub(super) fn unlayer(&self) {
        crate::video::unlayer(&self.player.shared, self.player.surface());
    }

    /// The clip time of the newest picture the decoder presented on the layer.
    pub(super) fn presented(&self) -> Option<f64> {
        let native = self.player.shared.native.lock().unwrap();
        let pts = native.pipe.as_ref()?.lock().layer_presented.map(|(pts, _)| pts)?;
        let duration = native.facts.map_or(0.0, |f| f.meta.duration);
        Some(if duration > 0.0 { pts.rem_euclid(duration) } else { pts })
    }

    /// For each beat heard, how far the clock was past the beat's time when it was heard.
    pub(super) fn offsets(&self, heard: &[Heard], after: Instant) -> Vec<f64> {
        beats(heard)
            .into_iter()
            .filter(|t| *t >= after)
            .filter_map(|t| {
                let (_, anchor) = self.clocks.iter().rev().find(|(at, _)| *at <= t)?;
                let media = anchor.as_ref()?.position(t);
                Some(media - media.round())
            })
            .collect()
    }
}

/// The frame number the fixture's bars spell in a drawn picture.
fn bars(rgba: &[u8]) -> u32 {
    let (w, h) = (VIEW.0 as usize, VIEW.1 as usize);
    (0..BARS).fold(0, |n, bit| {
        let x = ((bit as f64 + 0.5) * w as f64 / f64::from(BARS)) as usize;
        let lit = rgba[(h / 2 * w + x) * 4] > 128;
        n | (u32::from(lit) << bit)
    })
}

/// Root mean square of the first channel heard between `from` and `to`.
pub(super) fn rms(heard: &[Heard], from: Instant, to: Instant) -> f64 {
    let (mut sum, mut n) = (0.0, 0usize);
    for (t, x) in frames(heard) {
        if t >= from && t < to {
            sum += f64::from(x) * f64::from(x);
            n += 1;
        }
    }
    if n == 0 { 0.0 } else { (sum / n as f64).sqrt() }
}

/// When the first loud frame of a stretch of sound was heard.
pub(super) fn first_sound(heard: &[Heard], after: Instant) -> Option<Instant> {
    frames(heard).find(|(t, x)| *t >= after && x.abs() > 0.05).map(|(t, _)| t)
}

/// When each of the fixture's beats (a loud tone at every whole second) started to be heard.
fn beats(heard: &[Heard]) -> Vec<Instant> {
    let mut found = Vec::new();
    let mut loud: Option<Instant> = None;
    for (t, x) in frames(heard) {
        if x.abs() > 0.5 {
            if loud.is_none_or(|l| t.saturating_duration_since(l) > Duration::from_millis(300)) {
                found.push(t);
            }
            loud = Some(t);
        }
    }
    found
}

/// Every frame heard, first channel, with when it was heard.
fn frames(heard: &[Heard]) -> impl Iterator<Item = (Instant, f32)> + '_ {
    heard.iter().flat_map(|h| h.samples.chunks(h.channels).enumerate().map(move |(i, f)| (h.at + Duration::from_secs_f64(i as f64 / h.rate), f[0])))
}
