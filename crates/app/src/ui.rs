use std::collections::VecDeque;
use std::time::Duration;

use crossbeam_channel::Sender;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    EngineCmd, EngineStatus, FeedItem, NoticeLevel, RecordingEntry, UiMsg, reveal,
};
use rune::{TextEdit, Tx};

mod format;
mod preview;

#[cfg(test)]
mod tests;

pub(crate) use format::{format_clock, format_date, format_size};

type Scene = (&'static str, fn(&mut Ui));

pub(crate) const SCENES: &[Scene] = &[
    ("idle", |ui| {
        ui.status = EngineStatus::Idle;
        ui.feed.clear();
    }),
    ("waiting", |ui| {
        ui.status = EngineStatus::Waiting;
        ui.feed.clear();
    }),
    ("recording", |ui| ui.screen = Screen::Status),
    ("finalizing", |ui| ui.status = EngineStatus::Finalizing),
    ("error", |ui| {
        ui.status = EngineStatus::Error("The game window could not be captured.".into());
        ui.feed.clear();
    }),
    ("recordings", |ui| ui.screen = Screen::Recordings),
    ("recordings-empty", |ui| {
        ui.screen = Screen::Recordings;
        ui.recordings.clear();
    }),
    ("settings", |ui| ui.screen = Screen::Settings),
    ("review", |ui| {
        ui.review = ui.recordings.first().map(|r| r.meta.id.clone());
        ui.screen = Screen::Review;
    }),
];

pub(crate) const MIN_CLIP_SECS: u32 = 1;
pub(crate) const MAX_CLIP_SECS: u32 = 120;
pub(crate) const FEED_CAP: usize = 500;
pub(crate) const TEST_RECORDING_SECS: u32 = 10;
const NOTICE_SECS: f64 = 4.0;
const TICK: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Screen {
    #[default]
    Status,
    Recordings,
    Settings,
    Review,
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
    pub settings: Settings,
    pub notice: Option<(Notice, f64)>,
    pub cmd: Option<Sender<EngineCmd>>,
    pub recording_since_clock: Option<f64>,
    pub elapsed_secs: u32,
    pub review: Option<String>,
    pub resolution_open: bool,
    pub pre_secs: TextEdit,
    pub post_secs: TextEdit,
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
            screen: Screen::Status,
            status: EngineStatus::Idle,
            feed: VecDeque::new(),
            recordings: Vec::new(),
            notice: None,
            cmd: None,
            recording_since_clock: None,
            elapsed_secs: 0,
            review: None,
            resolution_open: false,
            pre_secs: TextEdit::new(settings.clips.pre_secs.to_string()),
            post_secs: TextEdit::new(settings.clips.post_secs.to_string()),
            settings,
        }
    }

    pub(crate) fn send(&self, cmd: EngineCmd) {
        if let Some(sender) = &self.cmd
            && sender.send(cmd).is_err()
        {
            tracing::warn!("engine stopped");
        }
    }

    pub(crate) fn go(&mut self, screen: Screen) {
        self.screen = screen;
        if screen == Screen::Recordings {
            self.send(EngineCmd::RescanLibrary);
        }
    }

    pub(crate) fn open_review(&mut self, id: String) {
        self.review = Some(id);
        self.screen = Screen::Review;
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
        }
    }

    fn apply_status(&mut self, status: EngineStatus, tx: &mut Tx) {
        let was_recording = matches!(self.status, EngineStatus::Recording { .. });
        match &status {
            EngineStatus::Recording { .. } if !was_recording => {
                self.feed.clear();
                self.recording_since_clock = Some(tx.clock());
                self.elapsed_secs = 0;
                tx.wake_after(TICK);
            }
            EngineStatus::Recording { .. } => {}
            _ => self.recording_since_clock = None,
        }
        self.status = status;
    }

    fn show_notice(&mut self, level: NoticeLevel, text: String, tx: &mut Tx) {
        let until = tx.clock() + NOTICE_SECS;
        self.notice = Some((Notice { level, text }, until));
        tx.wake_at(until);
    }

    pub(crate) fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        if let Some((_, until)) = &self.notice
            && clock >= *until
        {
            self.notice = None;
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
