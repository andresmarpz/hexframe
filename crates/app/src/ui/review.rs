mod deaths;
pub(crate) mod event_detail;
mod events;
pub(crate) mod jump;
mod player;
mod scrub;
mod seek_preview;
mod sound;
mod trace;

use std::path::{Path, PathBuf};

use greyscreen_core::categories::{CategorySet, EventCategory, categorize};
use greyscreen_core::library::storyboard::Storyboard;
use greyscreen_engine::RecordingEntry;
use rune::PlayerState;

pub(crate) use deaths::{DeathReviewState, title_line};
pub(crate) use events::EventHover;
pub(crate) use player::{PlayerFactory, ReviewPlayer, rune_player};
pub(crate) use scrub::Scrub;
pub(crate) use sound::{NORMAL_SPEED, SPEEDS, Sound, VolumeHover};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    Game,
    Clip(usize),
}

const FRAME_SECS: f64 = 1.0 / 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PanelTab {
    #[default]
    Events,
    Clips,
}

#[derive(Debug)]
pub(crate) struct ReviewState {
    pub entry: RecordingEntry,
    pub player: Option<Box<dyn ReviewPlayer>>,
    pub source: Source,
    pub track_width: f32,
    pub timeline_rect: [f32; 4],
    pub shown_state: Option<PlayerState>,
    pub shown_time: f64,
    pub settling: bool,
    pub theater: bool,
    pub full_screen: bool,
    pub controls_visible: bool,
    pub last_move: f64,
    pub hovered: Option<EventHover>,
    pub tab: PanelTab,
    pub categories: Vec<EventCategory>,
    pub filter: CategorySet,
    pub default_filter: CategorySet,
    pub filter_menu: bool,
    pub speed: usize,
    pub muted: bool,
    pub volume: f32,
    pub speed_menu: bool,
    pub health_open: bool,
    pub more_menu: bool,
    pub volume_hover: VolumeHover,
    pub drawer: bool,
    pub death: Option<DeathReviewState>,
    pub scrub: Option<Scrub>,
    pub storyboard: Option<Storyboard>,
    pub seek_hover: Option<f64>,
    /// The app clock at the last poll.
    pub clock: f64,
    pub file: PathBuf,
    pub watch: trace::Watch,
}

fn open_player(path: &Path, make: PlayerFactory) -> Option<Box<dyn ReviewPlayer>> {
    path.is_file().then(|| trace::traced(path, make(path)))
}

const CONTROLS_HIDE_SECS: f64 = 2.5;
const HALF_FRAME_SECS: f64 = 1.0 / 120.0;

impl ReviewState {
    pub(crate) fn open(
        entry: RecordingEntry,
        make: PlayerFactory,
        sound: Sound,
        default_filter: CategorySet,
    ) -> Self {
        let path = entry.folder.join(&entry.meta.video);
        let player = open_player(&path, make);
        if let Some(player) = &player {
            player.play();
        }
        let categories = categorize(&entry.meta.events, &entry.meta.game.info);
        let filter = entry.meta.review.filter.unwrap_or(default_filter);
        let mut review = ReviewState {
            entry,
            player,
            source: Source::Game,
            track_width: 0.0,
            timeline_rect: [0.0; 4],
            shown_state: None,
            shown_time: 0.0,
            settling: false,
            theater: false,
            full_screen: false,
            controls_visible: true,
            last_move: 0.0,
            hovered: None,
            tab: PanelTab::Events,
            categories,
            filter,
            default_filter,
            filter_menu: false,
            speed: NORMAL_SPEED,
            muted: sound.muted,
            volume: sound.volume,
            speed_menu: false,
            health_open: false,
            more_menu: false,
            volume_hover: VolumeHover::default(),
            drawer: false,
            death: None,
            scrub: None,
            storyboard: None,
            seek_hover: None,
            clock: 0.0,
            file: path,
            watch: trace::Watch::default(),
        };
        review.load_storyboard();
        review.apply_audio();
        review.fall_back_to_first_clip(make);
        review
    }

    fn fall_back_to_first_clip(&mut self, make: PlayerFactory) {
        if self.player.is_none() && !self.entry.meta.clips.is_empty() {
            self.play_clip(0, make);
        }
    }

    pub(crate) fn can_export_moment(&self) -> bool {
        self.source == Source::Game && self.player.is_some()
    }

    pub(crate) fn speed_rate(&self) -> f32 {
        SPEEDS[self.speed]
    }

    pub(crate) fn step_frame(&mut self, direction: f64) {
        if !self.is_playing() {
            self.seek(self.time() + direction * FRAME_SECS);
        }
    }

    pub(crate) fn active_clip(&self) -> Option<usize> {
        match self.source {
            Source::Clip(i) => Some(i),
            Source::Game => {
                let now = self.time();
                self.entry
                    .meta
                    .clips
                    .iter()
                    .position(|c| c.start_secs <= now && now <= c.end_secs)
            }
        }
    }

    fn apply_audio(&self) {
        let Some(player) = &self.player else { return };
        if self.speed != NORMAL_SPEED {
            player.set_rate(self.speed_rate());
        }
        if self.volume < 1.0 {
            player.set_volume(self.volume);
        }
        if self.muted {
            player.set_muted(true);
        }
    }

    pub(crate) fn wake_controls(&mut self, clock: f64) {
        self.last_move = clock;
        self.controls_visible = true;
    }

    pub(crate) fn hide_idle_controls(&mut self, clock: f64) -> bool {
        let idle = clock - self.last_move >= CONTROLS_HIDE_SECS;
        if self.theater
            && self.controls_visible
            && self.is_playing()
            && idle
            && !self.controls_menu_open()
        {
            self.controls_visible = false;
            self.hovered = None;
            return true;
        }
        false
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
        if let Some(scrub) = &self.scrub {
            return scrub.target;
        }
        self.player.as_ref().map_or(0.0, |p| p.time())
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.is_playing())
    }

    pub(crate) fn seek(&mut self, secs: f64) {
        if let Some(player) = &self.player {
            player.seek(secs.clamp(0.0, self.duration().max(0.0)));
            self.settling = true;
            if let Some(death) = &mut self.death {
                death.finished = false;
            }
        }
    }

    pub(crate) fn pause(&mut self) {
        if let Some(player) = self.player.as_ref().filter(|p| p.is_playing()) {
            player.pause();
        }
    }

    pub(crate) fn toggle(&mut self) {
        if self.death_finished() {
            self.replay_death();
        } else if self.is_playing() {
            self.pause();
        } else if self.player.is_some() {
            if self.held_at_end() {
                self.seek(0.0);
            }
            if let Some(player) = &self.player {
                player.play();
            }
        }
    }

    fn held_at_end(&self) -> bool {
        let duration = self.duration();
        duration > 0.0 && self.time() >= duration - HALF_FRAME_SECS
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
        self.scrub = None;
        self.file = path.to_owned();
        self.player = open_player(path, make);
        if let Some(player) = &self.player {
            player.play();
        }
        self.apply_audio();
        self.shown_state = None;
        self.shown_time = 0.0;
        self.settling = false;
        self.hovered = None;
        self.filter_menu = false;
        self.more_menu = false;
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
        active || self.settling || self.scrubbing()
    }

    pub(crate) fn tick(&mut self, clock: f64) {
        self.clock = clock;
        self.scrub_tick(clock);
        if let Some(player) = &self.player {
            self.watch
                .check(clock, player.as_ref(), self.scrub.is_some(), &self.file);
        }
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
