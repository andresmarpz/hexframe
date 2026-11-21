use std::fmt::Debug;
use std::path::Path;

use greyscreen_core::library::EventEntry;
use greyscreen_engine::RecordingEntry;
use rune::{El, Player, PlayerState, SurfaceFit};

pub(crate) trait ReviewPlayer: Debug {
    fn play(&self);
    fn pause(&self);
    fn seek(&self, secs: f64);
    fn time(&self) -> f64;
    fn duration(&self) -> Option<f64>;
    fn state(&self) -> PlayerState;
    fn set_rate(&self, rate: f32);
    fn set_muted(&self, muted: bool);
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

    fn set_rate(&self, rate: f32) {
        Player::set_rate(self, rate);
    }

    fn set_muted(&self, muted: bool) {
        Player::set_muted(self, muted);
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

pub(crate) const SPEEDS: [f32; 5] = [0.25, 0.5, 1.0, 1.5, 2.0];
const NORMAL_SPEED: usize = 2;
const FRAME_SECS: f64 = 1.0 / 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PanelTab {
    #[default]
    Events,
    Clips,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EventFilter {
    #[default]
    All,
    Deaths,
    Kills,
    Objectives,
}

impl EventFilter {
    pub(crate) const EVERY: [EventFilter; 4] = [
        EventFilter::All,
        EventFilter::Deaths,
        EventFilter::Kills,
        EventFilter::Objectives,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            EventFilter::All => "All",
            EventFilter::Deaths => "Deaths",
            EventFilter::Kills => "Kills",
            EventFilter::Objectives => "Objectives",
        }
    }

    pub(crate) fn matches(self, event: &EventEntry) -> bool {
        let any = |wanted: fn(&str) -> bool| event.highlights.iter().any(|h| wanted(h));
        match self {
            EventFilter::All => true,
            EventFilter::Deaths => any(|h| h == "MyDeath"),
            EventFilter::Kills => any(is_kill_label),
            EventFilter::Objectives => any(is_objective_label),
        }
    }
}

fn is_kill_label(label: &str) -> bool {
    matches!(label, "MyKill" | "MyAssist" | "FirstBlood" | "Ace") || label.starts_with("Multikill")
}

fn is_objective_label(label: &str) -> bool {
    let label = label.strip_prefix("Enemy").unwrap_or(label);
    matches!(
        label,
        "Baron" | "Dragon" | "Herald" | "VoidGrub" | "Turret" | "Inhibitor"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventHover {
    pub event_id: u32,
    pub anchor: String,
    pub theater: bool,
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
    pub theater: bool,
    pub controls_visible: bool,
    pub last_move: f64,
    pub hovered: Option<EventHover>,
    pub tab: PanelTab,
    pub filter: EventFilter,
    pub speed: usize,
    pub muted: bool,
    pub drawer: bool,
}

const CONTROLS_HIDE_SECS: f64 = 2.5;

impl ReviewState {
    pub(crate) fn open(entry: RecordingEntry, make: PlayerFactory) -> Self {
        let path = entry.folder.join(&entry.meta.video);
        let player = path.is_file().then(|| make(&path));
        if let Some(player) = &player {
            player.play();
        }
        let review = ReviewState {
            entry,
            player,
            source: Source::Game,
            track_width: 0.0,
            shown_state: None,
            shown_time: 0.0,
            settling: false,
            theater: false,
            controls_visible: true,
            last_move: 0.0,
            hovered: None,
            tab: PanelTab::Events,
            filter: EventFilter::All,
            speed: NORMAL_SPEED,
            muted: false,
            drawer: false,
        };
        review.apply_audio();
        review
    }

    pub(crate) fn speed_rate(&self) -> f32 {
        SPEEDS[self.speed]
    }

    pub(crate) fn cycle_speed(&mut self) {
        self.speed = (self.speed + 1) % SPEEDS.len();
        if let Some(player) = &self.player {
            player.set_rate(self.speed_rate());
        }
    }

    pub(crate) fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Some(player) = &self.player {
            player.set_muted(self.muted);
        }
    }

    pub(crate) fn step_frame(&mut self, direction: f64) {
        if !self.is_playing() {
            self.seek(self.time() + direction * FRAME_SECS);
        }
    }

    pub(crate) fn shown_events(&self) -> impl Iterator<Item = &EventEntry> {
        let filter = self.filter;
        self.entry
            .meta
            .events
            .iter()
            .filter(move |e| filter.matches(e))
    }

    pub(crate) fn active_event(&self) -> Option<u32> {
        if self.source != Source::Game {
            return None;
        }
        let now = self.time();
        self.shown_events()
            .filter(|e| e.rec_secs <= now)
            .max_by(|a, b| a.rec_secs.total_cmp(&b.rec_secs))
            .map(|e| e.id)
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
        if self.theater && self.controls_visible && self.is_playing() && idle {
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
        self.apply_audio();
        self.shown_state = None;
        self.shown_time = 0.0;
        self.settling = false;
        self.hovered = None;
    }

    pub(crate) fn hover_event(&mut self, event_id: u32, anchor: &str, inside: bool) {
        if inside {
            self.hovered = Some(EventHover {
                event_id,
                anchor: anchor.to_owned(),
                theater: self.theater,
            });
        } else if self
            .hovered
            .as_ref()
            .is_some_and(|h| h.event_id == event_id && h.anchor == anchor)
        {
            self.hovered = None;
        }
    }

    pub(crate) fn hovered_event(&self) -> Option<(&EventEntry, &str)> {
        let hover = self
            .hovered
            .as_ref()
            .filter(|h| h.theater == self.theater)?;
        let event = self
            .entry
            .meta
            .events
            .iter()
            .find(|e| e.id == hover.event_id)?;
        Some((event, hover.anchor.as_str()))
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
