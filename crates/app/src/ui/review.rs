use std::fmt::Debug;
use std::path::Path;

use greyscreen_engine::RecordingEntry;
use rune::{El, Player, PlayerState, SurfaceFit};

pub(crate) trait ReviewPlayer: Debug {
    fn play(&self);
    fn pause(&self);
    fn seek(&self, secs: f64);
    fn time(&self) -> f64;
    fn duration(&self) -> Option<f64>;
    fn state(&self) -> PlayerState;
    fn show(&self, frame: El) -> El;
}

impl ReviewPlayer for Player {
    fn play(&self) {
        Player::play(self);
    }

    fn pause(&self) {
        Player::pause(self);
    }

    fn seek(&self, secs: f64) {
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

    fn show(&self, frame: El) -> El {
        frame.video(self, SurfaceFit::Fit)
    }
}

pub(crate) type PlayerFactory = fn(&Path) -> Box<dyn ReviewPlayer>;

pub(crate) fn rune_player(path: &Path) -> Box<dyn ReviewPlayer> {
    let player = Player::file(path);
    player.set_muted(false);
    Box::new(player)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    Game,
    Clip(usize),
}

#[derive(Debug)]
pub(crate) struct ReviewState {
    pub entry: RecordingEntry,
    pub player: Option<Box<dyn ReviewPlayer>>,
    pub source: Source,
    pub track_width: f32,
    pub shown_state: Option<PlayerState>,
    pub shown_time: f64,
    pub settling: bool,
}

impl ReviewState {
    pub(crate) fn open(entry: RecordingEntry, make: PlayerFactory) -> Self {
        let path = entry.folder.join(&entry.meta.video);
        let player = path.is_file().then(|| make(&path));
        if let Some(player) = &player {
            player.play();
        }
        ReviewState {
            entry,
            player,
            source: Source::Game,
            track_width: 0.0,
            shown_state: None,
            shown_time: 0.0,
            settling: false,
        }
    }

    pub(crate) fn duration(&self) -> f64 {
        let fallback = match self.source {
            Source::Game => self.entry.meta.duration_secs,
            Source::Clip(i) => self
                .entry
                .meta
                .clips
                .get(i)
                .map_or(0.0, |c| c.end_secs - c.start_secs),
        };
        self.player
            .as_ref()
            .and_then(|p| p.duration())
            .filter(|d| *d > 0.0)
            .unwrap_or(fallback)
    }

    pub(crate) fn time(&self) -> f64 {
        self.player.as_ref().map_or(0.0, |p| p.time())
    }

    pub(crate) fn is_playing(&self) -> bool {
        matches!(
            self.player.as_ref().map(|p| p.state()),
            Some(PlayerState::Playing)
        )
    }

    pub(crate) fn seek(&mut self, secs: f64) {
        if let Some(player) = &self.player {
            player.seek(secs.clamp(0.0, self.duration()));
            self.settling = true;
        }
    }

    pub(crate) fn toggle(&self) {
        if let Some(player) = &self.player {
            if self.is_playing() {
                player.pause();
            } else {
                player.play();
            }
        }
    }

    pub(crate) fn play_clip(&mut self, index: usize, make: PlayerFactory) {
        let Some(clip) = self.entry.meta.clips.get(index) else {
            return;
        };
        let path = self.entry.folder.join(&clip.file);
        self.swap(Source::Clip(index), &path, make);
    }

    pub(crate) fn play_game(&mut self, make: PlayerFactory) {
        let path = self.entry.folder.join(&self.entry.meta.video);
        self.swap(Source::Game, &path, make);
    }

    fn swap(&mut self, source: Source, path: &Path, make: PlayerFactory) {
        self.source = source;
        self.player = path.is_file().then(|| make(path));
        if let Some(player) = &self.player {
            player.play();
        }
        self.shown_state = None;
        self.shown_time = 0.0;
        self.settling = false;
    }

    pub(crate) fn needs_ticks(&mut self) -> bool {
        let Some(player) = &self.player else {
            return false;
        };
        let state = player.state();
        let time = player.time();
        let active = matches!(state, PlayerState::Loading | PlayerState::Playing);
        if self.settling && !active && time == self.shown_time {
            self.settling = false;
        }
        active || self.settling
    }

    pub(crate) fn changed_since_shown(&mut self) -> bool {
        let Some(player) = &self.player else {
            return false;
        };
        let (state, time) = (player.state(), player.time());
        let changed = self.shown_state.as_ref() != Some(&state) || self.shown_time != time;
        self.shown_state = Some(state);
        self.shown_time = time;
        changed
    }
}
