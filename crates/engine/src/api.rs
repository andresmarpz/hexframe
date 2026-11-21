use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::highlight::Highlight;
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_core::media::MediaSource;
use greyscreen_core::settings::Settings;
use greyscreen_league::{GameLocator, LeagueMsg, LiveClient, Poller};

use crate::control;

pub use greyscreen_core::session::Status as EngineStatus;

const DEFAULT_LIVE_URL: &str = "https://127.0.0.1:2999";
const LIVE_URL_ENV: &str = "GREYSCREEN_LIVE_URL";

#[derive(Debug, Clone, PartialEq)]
pub enum EngineCmd {
    ApplySettings(Settings),
    StartTestRecording { secs: u32 },
    StopRecording,
    RescanLibrary,
    SaveClipNow,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedItem {
    pub id: u32,
    pub rec_secs: f64,
    pub game_secs: f64,
    pub text: String,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordingEntry {
    pub meta: RecordingMeta,
    pub folder: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipEntry {
    pub clip: library::ClipEntry,
    pub folder: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum UiMsg {
    Status(EngineStatus),
    Feed(FeedItem),
    ClipSaved(ClipEntry),
    RecordingSaved(RecordingEntry),
    Library(Vec<RecordingEntry>),
    Notice { level: NoticeLevel, text: String },
}

pub type UiSink = Box<dyn Fn(UiMsg) + Send + Sync>;
pub type LeagueGuard = Box<dyn Send>;
pub type LeagueFactory = Box<dyn Fn(Sender<LeagueMsg>) -> LeagueGuard + Send>;

pub fn league_factory_default() -> LeagueFactory {
    Box::new(|out| {
        let url = std::env::var(LIVE_URL_ENV).unwrap_or_else(|_| DEFAULT_LIVE_URL.to_owned());
        let client = LiveClient::new(&url);
        Box::new(Poller::spawn(client, default_locator(), out))
    })
}

#[cfg(windows)]
fn default_locator() -> Box<dyn GameLocator> {
    Box::new(greyscreen_league::locator::WindowLocator::new())
}

#[cfg(not(windows))]
fn default_locator() -> Box<dyn GameLocator> {
    Box::new(greyscreen_league::AlwaysPresent)
}

#[derive(Debug)]
pub struct Engine;

impl Engine {
    pub fn start(
        mut settings: Settings,
        media: Arc<dyn MediaSource>,
        league: LeagueFactory,
        sink: UiSink,
    ) -> EngineHandle {
        settings.validate();
        let (cmd, cmd_rx) = unbounded();
        let (league_tx, league_rx) = unbounded();
        let guard = league(league_tx);
        let thread = control::spawn(settings, media, guard, league_rx, cmd_rx, Arc::from(sink));
        EngineHandle { cmd, thread }
    }
}

#[derive(Debug)]
pub struct EngineHandle {
    pub cmd: Sender<EngineCmd>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        if self.cmd.send(EngineCmd::Shutdown).is_err() {
            tracing::debug!("engine already stopped");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("engine control thread panicked");
        }
    }
}
