use std::fmt::Debug;
use std::path::Path;

use rune::{El, Player, PlayerState, SurfaceFit};

pub(crate) trait ReviewPlayer: Debug {
    fn play(&self);
    fn pause(&self);
    fn seek(&self, secs: f64);
    fn time(&self) -> f64;
    fn duration(&self) -> Option<f64>;
    fn state(&self) -> PlayerState;
    fn is_playing(&self) -> bool;
    fn set_rate(&self, rate: f32);
    fn set_muted(&self, muted: bool);
    fn set_volume(&self, volume: f32);
    fn show(&self, frame: El) -> El;
    fn follow(&self, _clock: f64) {}
}

fn log_call(player: &Player, call: std::fmt::Arguments) {
    let kind = if player.is_audio() { "sound" } else { "video" };
    tracing::debug!(
        target: crate::logging::PLAYBACK_TARGET,
        "{kind} {call} at {:.3} ({:?})",
        Player::time(player),
        Player::state(player)
    );
}

impl ReviewPlayer for Player {
    fn play(&self) {
        log_call(self, format_args!("play"));
        Player::play(self);
    }

    fn pause(&self) {
        log_call(self, format_args!("pause"));
        Player::pause(self);
    }

    fn seek(&self, secs: f64) {
        log_call(self, format_args!("seek {secs:.3}"));
        Player::seek(self, secs);
    }

    fn time(&self) -> f64 {
        Player::time(self)
    }

    fn duration(&self) -> Option<f64> {
        Player::duration(self)
    }

    fn state(&self) -> PlayerState {
        Player::state(self)
    }

    fn is_playing(&self) -> bool {
        Player::is_playing(self)
    }

    fn set_rate(&self, rate: f32) {
        log_call(self, format_args!("rate {rate}"));
        Player::set_rate(self, rate);
    }

    fn set_muted(&self, muted: bool) {
        log_call(self, format_args!("muted {muted}"));
        Player::set_muted(self, muted);
    }

    fn set_volume(&self, volume: f32) {
        log_call(self, format_args!("volume {volume:.2}"));
        Player::set_volume(self, volume);
    }

    fn show(&self, frame: El) -> El {
        frame.video(self, SurfaceFit::Fit)
    }
}

pub(crate) type PlayerFactory = fn(&Path) -> Box<dyn ReviewPlayer>;

#[cfg(not(windows))]
pub(crate) fn rune_player(path: &Path) -> Box<dyn ReviewPlayer> {
    let player = Player::file(path);
    player.set_muted(false);
    Box::new(player)
}

#[cfg(windows)]
pub(crate) fn rune_player(path: &Path) -> Box<dyn ReviewPlayer> {
    let sound = Player::audio(path);
    sound.preload();
    Box::new(super::paired_sound::PairedSound::new(
        Box::new(Player::file(path)),
        Box::new(sound),
    ))
}
