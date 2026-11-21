use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;

use greyscreen_core::settings::Settings;
use rune::TextEdit;

mod preview;

type Scene = (&'static str, fn(&mut Ui));

pub(crate) const SCENES: &[Scene] = &[
    ("idle", |ui| {
        ui.status = PreviewStatus::Idle;
        ui.feed.clear();
    }),
    ("recording", |ui| ui.screen = Screen::Status),
    ("recordings", |ui| ui.screen = Screen::Recordings),
    ("settings", |ui| ui.screen = Screen::Settings),
];

pub(crate) const MIN_CLIP_SECS: u32 = 1;
pub(crate) const MAX_CLIP_SECS: u32 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Screen {
    #[default]
    Status,
    Recordings,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreviewStatus {
    Idle,
    Recording {
        elapsed_secs: u32,
        champion: String,
        mode: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeedKind {
    Kill,
    Death,
    Assist,
    Multikill,
    FirstBlood,
    Dragon,
    Herald,
    Baron,
    Turret,
    Inhibitor,
    Ace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedRow {
    pub id: u32,
    pub game_secs: u32,
    pub kind: FeedKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultKind {
    Win,
    Loss,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecRow {
    pub id: String,
    pub started: String,
    pub champion: String,
    pub mode: String,
    pub result: ResultKind,
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct Ui {
    pub screen: Screen,
    pub status: PreviewStatus,
    pub feed: Vec<FeedRow>,
    pub recordings: Vec<RecRow>,
    pub settings: Settings,
    pub resolution_open: bool,
    pub pre_secs: TextEdit,
    pub post_secs: TextEdit,
    saver: Option<Sender<Settings>>,
}

impl Ui {
    pub(crate) fn preview() -> Ui {
        preview::ui()
    }

    pub(crate) fn blank(settings: Settings) -> Ui {
        Ui {
            screen: Screen::Status,
            status: PreviewStatus::Idle,
            feed: Vec::new(),
            recordings: Vec::new(),
            pre_secs: TextEdit::new(settings.clips.pre_secs.to_string()),
            post_secs: TextEdit::new(settings.clips.post_secs.to_string()),
            settings,
            resolution_open: false,
            saver: None,
        }
    }

    pub(crate) fn load_settings(&mut self, settings: Settings, path: PathBuf) {
        self.pre_secs = TextEdit::new(settings.clips.pre_secs.to_string());
        self.post_secs = TextEdit::new(settings.clips.post_secs.to_string());
        self.settings = settings;
        self.saver = Some(spawn_saver(path));
    }

    pub(crate) fn change(&mut self, edit: impl FnOnce(&mut Settings)) {
        edit(&mut self.settings);
        self.settings.validate();
        if let Some(saver) = &self.saver
            && saver.send(self.settings.clone()).is_err()
        {
            tracing::warn!("settings saver stopped");
        }
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

fn spawn_saver(path: PathBuf) -> Sender<Settings> {
    let (tx, rx) = mpsc::channel::<Settings>();
    thread::spawn(move || {
        for settings in rx {
            if let Err(err) = settings.save(&path) {
                tracing::warn!(path = %path.display(), %err, "cannot save settings");
            }
        }
    });
    tx
}

pub(crate) fn format_clock(secs: u32) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

pub(crate) fn format_size(bytes: u64) -> String {
    const MB: f64 = 1_000_000.0;
    let mb = bytes as f64 / MB;
    if mb >= 1000.0 {
        format!("{:.1} GB", mb / 1000.0)
    } else {
        format!("{mb:.0} MB")
    }
}
