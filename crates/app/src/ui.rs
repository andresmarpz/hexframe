use std::collections::VecDeque;
use std::time::Duration;

use crossbeam_channel::Sender;
use greyscreen_core::keybinds::Action;
use greyscreen_core::settings::Settings;
use greyscreen_core::update::UpdateInfo;
use greyscreen_engine::{
    EngineCmd, EngineStatus, FeedItem, NoticeLevel, RecordingEntry, UiMsg, reveal,
};
use rune::{TextEdit, Tx};

pub(crate) mod event_detail;
mod format;
mod jump;
pub(crate) mod key_event;
mod keybinds;
mod keys;
pub(crate) mod library_format;
mod preview;
pub(crate) mod recordings_filter;
mod review;
mod scenes;
mod tone;
mod update;

#[cfg(test)]
pub(crate) mod fake_player;
#[cfg(test)]
mod tests;

pub(crate) use format::{format_clock, format_size, format_size_rate};
pub(crate) use jump::seek_target;
pub(crate) use recordings_filter::{RecordingsFilter, ResultFilter, modes_present};
pub(crate) use review::{EventFilter, PanelTab, PlayerFactory, ReviewState, Source};
pub(crate) use scenes::SCENES;
pub(crate) use tone::tone_color;
pub(crate) use update::UpdateUi;

#[cfg(test)]
pub(crate) fn review_entry() -> RecordingEntry {
    preview::review_entry()
}

pub(crate) const MIN_CLIP_SECS: u32 = 1;
pub(crate) const MAX_CLIP_SECS: u32 = 120;
pub(crate) const FEED_CAP: usize = 500;
pub(crate) const TEST_RECORDING_SECS: u32 = 10;
pub(crate) const STOP_CONFIRM_AFTER_SECS: u32 = 60;
const STOP_CONFIRM_SECS: f64 = 3.0;
const NOTICE_SECS: f64 = 4.0;
const TICK: Duration = Duration::from_millis(250);
const PLAYBACK_TICK: Duration = Duration::from_millis(33);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Screen {
    #[default]
    Home,
    Recordings,
    Settings,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SettingsTab {
    #[default]
    General,
    Keybinds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Notice {
    pub level: NoticeLevel,
    pub text: String,
}

#[derive(Debug)]
pub(crate) struct Ui {
    pub screen: Screen,
    pub status: EngineStatus,
    pub feed: VecDeque<FeedItem>,
    pub recordings: Vec<RecordingEntry>,
    pub filter: RecordingsFilter,
    pub last_game: Option<RecordingEntry>,
    pub settings: Settings,
    pub notice: Option<(Notice, f64)>,
    pub cmd: Option<Sender<EngineCmd>>,
    pub recording_since_clock: Option<f64>,
    pub elapsed_secs: u32,
    pub stop_confirm_until: Option<f64>,
    pub review: Option<ReviewState>,
    pub player_factory: PlayerFactory,
    pub resolution_open: bool,
    pub storage_open: bool,
    pub settings_tab: SettingsTab,
    pub rebinding: Option<Action>,
    pub bind_hint: Option<(Action, String)>,
    pub hotkey_errors: Vec<(Action, String)>,
    pub window_full_screen: bool,
    pub start_with_windows: bool,
    pub pre_secs: TextEdit,
    pub post_secs: TextEdit,
    pub update: UpdateUi,
    pub update_available: Option<UpdateInfo>,
}

impl Ui {
    pub(crate) fn preview() -> Ui {
        preview::ui()
    }

    pub(crate) fn live(settings: Settings, cmd: Sender<EngineCmd>) -> Ui {
        let mut ui = Ui::blank(settings);
        ui.cmd = Some(cmd);
        ui
    }

    pub(crate) fn blank(settings: Settings) -> Ui {
        Ui {
            screen: Screen::Home,
            status: EngineStatus::Idle,
            feed: VecDeque::new(),
            recordings: Vec::new(),
            filter: RecordingsFilter::default(),
            last_game: None,
            notice: None,
            cmd: None,
            recording_since_clock: None,
            elapsed_secs: 0,
            stop_confirm_until: None,
            review: None,
            player_factory: review::rune_player,
            resolution_open: false,
            storage_open: false,
            settings_tab: SettingsTab::General,
            rebinding: None,
            bind_hint: None,
            hotkey_errors: Vec::new(),
            window_full_screen: false,
            start_with_windows: false,
            pre_secs: TextEdit::new(settings.clips.pre_secs.to_string()),
            post_secs: TextEdit::new(settings.clips.post_secs.to_string()),
            update: UpdateUi::default(),
            update_available: None,
            settings,
        }
    }

    pub(crate) fn toggle_start_with_windows(&mut self) {
        self.start_with_windows = !self.start_with_windows;
        #[cfg(windows)]
        {
            let enabled = self.start_with_windows;
            std::thread::spawn(move || {
                if let Err(err) = greyscreen_capture::install::set_start_with_windows(enabled) {
                    tracing::warn!(%err, "cannot change start with Windows");
                }
            });
        }
    }

    pub(crate) fn send(&self, cmd: EngineCmd) {
        if let Some(sender) = &self.cmd
            && sender.send(cmd).is_err()
        {
            tracing::warn!("engine stopped");
        }
    }

    pub(crate) fn request_stop(&mut self, tx: &mut Tx) {
        let now = tx.clock();
        let confirmed = self.stop_confirm_until.is_some_and(|until| now < until);
        if self.elapsed_secs <= STOP_CONFIRM_AFTER_SECS || confirmed {
            self.stop_confirm_until = None;
            self.send(EngineCmd::StopRecording);
        } else {
            let until = now + STOP_CONFIRM_SECS;
            self.stop_confirm_until = Some(until);
            tx.wake_at(until);
        }
    }

    pub(crate) fn retry_capture(&self) {
        self.send(EngineCmd::Retry);
    }

    pub(crate) fn dismiss_error(&mut self) {
        if matches!(self.status, EngineStatus::Error(_)) {
            self.status = EngineStatus::Idle;
        }
    }

    pub(crate) fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.stop_rebinding();
        if screen != Screen::Review {
            self.review = None;
        }
        if screen == Screen::Recordings {
            self.send(EngineCmd::RescanLibrary);
        }
    }

    pub(crate) fn open_review(&mut self, id: String) {
        if let Some(entry) = self.recordings.iter().find(|r| r.meta.id == id) {
            self.open_review_entry(entry.clone());
        }
    }

    pub(crate) fn open_last_game(&mut self) {
        if let Some(entry) = self.last_game.clone() {
            self.open_review_entry(entry);
        }
    }

    pub(crate) fn open_clip(&mut self, id: String, index: usize) {
        self.open_review(id);
        self.play_clip(index);
    }

    pub(crate) fn open_review_entry(&mut self, entry: RecordingEntry) {
        self.review = Some(ReviewState::open(entry, self.player_factory));
        self.screen = Screen::Review;
    }

    pub(crate) fn play_clip(&mut self, index: usize) {
        let make = self.player_factory;
        if let Some(review) = &mut self.review {
            review.play_clip(index, make);
        }
    }

    pub(crate) fn play_game(&mut self) {
        let make = self.player_factory;
        if let Some(review) = &mut self.review {
            review.play_game(make);
        }
    }

    pub(crate) fn review_seek(&mut self, secs: f64) {
        if let Some(review) = &mut self.review {
            review.seek(secs);
        }
    }

    pub(crate) fn reveal_output_dir(&self) {
        reveal(&self.settings.output_dir);
    }

    pub(crate) fn apply(&mut self, msg: UiMsg, tx: &mut Tx) {
        match msg {
            UiMsg::Status(status) => self.apply_status(status, tx),
            UiMsg::Feed(item) => {
                self.feed.push_back(item);
                while self.feed.len() > FEED_CAP {
                    self.feed.pop_front();
                }
            }
            UiMsg::ClipSaved(entry) => {
                let text = format!("Clip saved: {}", entry.clip.label);
                self.show_notice(NoticeLevel::Info, text, tx);
            }
            UiMsg::RecordingSaved(entry) => {
                let text = format!("Recording saved: {}", entry.meta.game.info.me.champion);
                self.last_game = Some(entry.clone());
                match self
                    .recordings
                    .iter_mut()
                    .find(|r| r.meta.id == entry.meta.id)
                {
                    Some(slot) => *slot = entry,
                    None => self.recordings.insert(0, entry),
                }
                self.show_notice(NoticeLevel::Info, text, tx);
            }
            UiMsg::Library(recordings) => self.recordings = recordings,
            UiMsg::Notice { level, text } => self.show_notice(level, text, tx),
            UiMsg::Update(state) => self.apply_update(state),
            UiMsg::HotkeyStatus { errors } => self.hotkey_errors = errors,
        }
    }

    fn apply_status(&mut self, status: EngineStatus, tx: &mut Tx) {
        let was_recording = matches!(self.status, EngineStatus::Recording { .. });
        match &status {
            EngineStatus::Recording { .. } if !was_recording => {
                self.feed.clear();
                self.last_game = None;
                self.recording_since_clock = Some(tx.clock());
                self.elapsed_secs = 0;
                tx.wake_after(TICK);
            }
            EngineStatus::Recording { .. } => {}
            EngineStatus::Idle | EngineStatus::GameRunning => {
                self.recording_since_clock = None;
                self.feed.clear();
            }
            _ => self.recording_since_clock = None,
        }
        if !matches!(status, EngineStatus::Recording { .. }) {
            self.stop_confirm_until = None;
        }
        self.status = status;
        self.install_if_idle();
    }

    pub(crate) fn show_notice(&mut self, level: NoticeLevel, text: String, tx: &mut Tx) {
        let until = tx.clock() + NOTICE_SECS;
        self.notice = Some((Notice { level, text }, until));
        tx.wake_at(until);
    }

    pub(crate) fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        self.sync_full_screen(tx);
        if let Some((_, until)) = &self.notice
            && clock >= *until
        {
            self.notice = None;
            changed = true;
        }
        if let Some(until) = self.stop_confirm_until
            && clock >= until
        {
            self.stop_confirm_until = None;
            changed = true;
        }
        if let Some(start) = self.recording_since_clock {
            let secs = (clock - start).max(0.0) as u32;
            if secs != self.elapsed_secs {
                self.elapsed_secs = secs;
                changed = true;
            }
            tx.wake_after(TICK);
        }
        if let Some(review) = &mut self.review {
            changed |= review.changed_since_shown();
            changed |= review.hide_idle_controls(clock);
            if review.needs_ticks() {
                tx.wake_after(PLAYBACK_TICK);
            }
        }
        changed
    }

    pub(crate) fn change(&mut self, edit: impl FnOnce(&mut Settings)) {
        edit(&mut self.settings);
        self.settings.validate();
        self.send(EngineCmd::ApplySettings(self.settings.clone()));
    }

    pub(crate) fn edit_clip_secs(&mut self) {
        if let Some(secs) = parse_clip_secs(&self.pre_secs.text) {
            self.change(|s| s.clips.pre_secs = secs);
        }
        if let Some(secs) = parse_clip_secs(&self.post_secs.text) {
            self.change(|s| s.clips.post_secs = secs);
        }
    }
}

pub(crate) fn parse_clip_secs(text: &str) -> Option<u32> {
    let secs: u32 = text.trim().parse().ok()?;
    (MIN_CLIP_SECS..=MAX_CLIP_SECS)
        .contains(&secs)
        .then_some(secs)
}
