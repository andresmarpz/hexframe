use std::collections::VecDeque;

use crossbeam_channel::Sender;
use greyscreen_core::disk::StorageUsage;
use greyscreen_core::encoder::EncoderCheck;
use greyscreen_core::keybinds::Action;
use greyscreen_core::settings::Settings;
use greyscreen_core::update::UpdateInfo;
use greyscreen_engine::{
    EngineCmd, EngineStatus, FeedItem, NoticeAction, NoticeLevel, RecordingEntry,
};
use rune::{PointerPress, TextEdit};

use crate::sfx::Sfx;

mod export;
mod feedback;
mod first_run;
mod format;
mod keys;
pub(crate) mod library;
mod preview;
pub(crate) mod review;
mod scenes;
mod session;
mod settings_sheet;
mod shell;
mod sounds;
mod update;

#[cfg(test)]
pub(crate) mod fake_player;
#[cfg(test)]
mod tests;

pub(crate) use export::{CopyState, Destination, ExportPhase, ExportUi, Handle as RangeHandle};
pub(crate) use feedback::{FeedbackPhase, FeedbackUi, MESSAGE_KEY as FEEDBACK_MESSAGE_KEY};
pub(crate) use first_run::TestRun;
pub(crate) use format::{format_clock, format_length, format_size, format_size_rate};
pub(crate) use keys::MAC;
pub(crate) use library::{Detail, ListItem, RecordingsFilter};
pub(crate) use review::jump::seek_target;
pub(crate) use review::{
    DeathReviewState, NORMAL_SPEED, PanelTab, PlayerFactory, ReviewState, SPEEDS, Sound, Source,
    VolumeHover, title_line,
};
pub(crate) use scenes::all as scene_list;
pub(crate) use settings_sheet::SettingsSection;
pub(crate) use shell::{DockState, PaneDrag, PaneEdge, Screen, WindowSizeWatch, remember_window};
pub(crate) use update::UpdateUi;

#[cfg(test)]
pub(crate) fn death_entry() -> RecordingEntry {
    preview::death_entry()
}

#[cfg(test)]
pub(crate) fn review_entry() -> RecordingEntry {
    preview::review_entry()
}

pub(crate) const MIN_CLIP_SECS: u32 = 1;
pub(crate) const MAX_CLIP_SECS: u32 = 120;
pub(crate) const FEED_CAP: usize = 500;
pub(crate) const TEST_RECORDING_SECS: u32 = 10;
pub(crate) const STOP_CONFIRM_AFTER_SECS: u32 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Notice {
    pub level: NoticeLevel,
    pub text: String,
    pub action: Option<NoticeAction>,
}

#[derive(Debug)]
pub(crate) struct Ui {
    pub screen: Screen,
    pub pane_drag: Option<PaneDrag>,
    pub window_watch: WindowSizeWatch,
    pub live_selected: bool,
    pub game_page: Option<String>,
    pub status: EngineStatus,
    pub feed: VecDeque<FeedItem>,
    pub recordings: Vec<RecordingEntry>,
    pub storage: Option<StorageUsage>,
    pub filter: RecordingsFilter,
    pub last_game: Option<RecordingEntry>,
    pub settings: Settings,
    pub notice: Option<(Notice, f64)>,
    pub cmd: Option<Sender<EngineCmd>>,
    pub recording_since_clock: Option<f64>,
    pub elapsed_secs: u32,
    pub stop_confirm_until: Option<f64>,
    pub review: Option<ReviewState>,
    pub row_menu: Option<(String, PointerPress)>,
    pub player_factory: PlayerFactory,
    pub display_sizes: fn() -> Vec<(u32, u32)>,
    pub sound: Sound,
    pub sfx: Sfx,
    pub resolution_open: bool,
    pub storage_open: bool,
    pub encoder_detail_open: bool,
    pub settings_open: bool,
    pub settings_section: SettingsSection,
    pub settings_search: TextEdit,
    pub clips_search: TextEdit,
    pub rebinding: Option<Action>,
    pub bind_hint: Option<(Action, String)>,
    pub hotkey_errors: Vec<(Action, String)>,
    pub window_full_screen: bool,
    pub start_with_windows: bool,
    pub pre_secs: TextEdit,
    pub post_secs: TextEdit,
    pub update: UpdateUi,
    pub update_available: Option<UpdateInfo>,
    pub death_note: TextEdit,
    pub dock: DockState,
    pub feedback: FeedbackUi,
    pub export: ExportUi,
    pub first_test: TestRun,
    pub tray: Option<crate::tray::Link>,
    pub encoder: Option<EncoderCheck>,
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
            pane_drag: None,
            window_watch: WindowSizeWatch::new(),
            live_selected: false,
            game_page: None,
            status: EngineStatus::Idle,
            feed: VecDeque::new(),
            recordings: Vec::new(),
            storage: None,
            filter: RecordingsFilter::default(),
            last_game: None,
            notice: None,
            cmd: None,
            recording_since_clock: None,
            elapsed_secs: 0,
            stop_confirm_until: None,
            review: None,
            row_menu: None,
            player_factory: review::rune_player,
            display_sizes: crate::platform::connected_display_sizes,
            sound: Sound::default(),
            sfx: Sfx::silent(),
            resolution_open: false,
            storage_open: false,
            encoder_detail_open: false,
            settings_open: false,
            settings_section: SettingsSection::default(),
            settings_search: TextEdit::default(),
            clips_search: TextEdit::default(),
            rebinding: None,
            bind_hint: None,
            hotkey_errors: Vec::new(),
            window_full_screen: false,
            start_with_windows: false,
            pre_secs: TextEdit::new(settings.clips.pre_secs.to_string()),
            post_secs: TextEdit::new(settings.clips.post_secs.to_string()),
            update: UpdateUi::default(),
            update_available: None,
            death_note: TextEdit::new(String::new()),
            dock: DockState::default(),
            feedback: FeedbackUi::default(),
            export: ExportUi::default(),
            first_test: TestRun::default(),
            tray: None,
            encoder: None,
            settings,
        }
    }

    pub(crate) fn toggle_start_with_windows(&mut self) {
        self.start_with_windows = !self.start_with_windows;
        if !rune::is_headless() {
            crate::platform::set_start_with_windows(self.start_with_windows);
        }
    }

    pub(crate) fn send(&self, cmd: EngineCmd) {
        if let Some(sender) = &self.cmd
            && sender.send(cmd).is_err()
        {
            tracing::warn!("engine stopped");
        }
    }

    pub(crate) fn open_review(&mut self, id: String) {
        if let Some(entry) = self.recordings.iter().find(|r| r.meta.id == id) {
            self.open_review_entry(entry.clone());
        }
    }

    pub(crate) fn open_clip(&mut self, id: String, index: usize) {
        self.open_review(id);
        self.play_clip(index);
    }

    pub(crate) fn open_last_game(&mut self) {
        if let Some(entry) = self.last_game.clone() {
            self.open_review_entry(entry);
        }
    }

    pub(crate) fn open_review_entry(&mut self, entry: RecordingEntry) {
        self.close_settings();
        self.game_page = None;
        let review = ReviewState::open(
            entry,
            self.player_factory,
            self.sound,
            self.settings.review_events,
        );
        if review.wants_storyboard() {
            self.send(EngineCmd::BuildStoryboard {
                folder: review.entry.folder.clone(),
            });
        }
        self.review = Some(review);
        self.live_selected = false;
        self.screen = Screen::Games;
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

    pub(crate) fn review_scrub(&mut self, window_x: f32) {
        if let Some(review) = &mut self.review {
            let [left, _, width, _] = review.timeline_rect;
            if width > 0.0 {
                let secs =
                    f64::from(((window_x - left) / width).clamp(0.0, 1.0)) * review.duration();
                review.scrub_to(secs);
            }
        }
    }

    pub(crate) fn review_scrub_to(&mut self, secs: f64) {
        if let Some(review) = &mut self.review {
            review.scrub_to(secs);
        }
    }

    pub(crate) fn review_scrub_end(&mut self) {
        if let Some(review) = &mut self.review {
            review.scrub_end();
        }
    }

    pub(crate) fn reveal_output_dir(&self) {
        crate::headless::reveal(&self.settings.output_dir);
    }

    pub(crate) fn change(&mut self, edit: impl FnOnce(&mut Settings)) {
        edit(&mut self.settings);
        self.settings.validate();
        self.sync_review_default();
        self.send(EngineCmd::ApplySettings(self.settings.clone()));
        self.sync_tray();
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
