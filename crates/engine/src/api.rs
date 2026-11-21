use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::encoder::EncoderCheck;
use greyscreen_core::export::{ExportEvent, ExportJob};
use greyscreen_core::feedback::{FeedbackDraft, FeedbackResult};
use greyscreen_core::highlight::Highlight;
use greyscreen_core::keybinds::Action;
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_core::media::MediaSource;
use greyscreen_core::paths;
use greyscreen_core::settings::Settings;
use greyscreen_core::update::UpdateState;
use greyscreen_league::{GameLocator, LeagueMsg, LiveClient, Poller};

use crate::control;
use crate::disk::{FreeSpace, SystemFreeSpace};
use crate::feedback::FeedbackConfig;
use crate::hotkey::HotkeyBackend;
use crate::updater::UpdateConfig;

pub use greyscreen_core::session::Status as EngineStatus;

const DEFAULT_LIVE_URL: &str = "https://127.0.0.1:2999";
const LIVE_URL_ENV: &str = "GREYSCREEN_LIVE_URL";

#[derive(Debug, Clone, PartialEq)]
pub enum EngineCmd {
    ApplySettings(Settings),
    StartTestRecording { secs: u32 },
    StopRecording,
    Retry,
    RescanLibrary,
    DeleteRecording { id: String },
    SaveClipNow,
    GlobalAction(Action),
    CheckForUpdates,
    InstallUpdate,
    SendFeedback(FeedbackDraft),
    Export(ExportJob),
    CancelExport,
    Shutdown,
}

pub const MANUAL_CLIP_LABEL: &str = "Manual";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeAction {
    Reveal(PathBuf),
    Link(String),
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
    pub size_bytes: u64,
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
    Notice {
        level: NoticeLevel,
        text: String,
    },
    NoticeWithAction {
        level: NoticeLevel,
        text: String,
        action: NoticeAction,
    },
    HotkeyStatus {
        errors: Vec<(Action, String)>,
    },
    Update(UpdateState),
    Feedback(FeedbackResult),
    Export(ExportEvent),
    Thumbnail(PathBuf),
    Encoder(EncoderCheck),
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

pub struct EngineConfig {
    pub settings: Settings,
    pub media: Arc<dyn MediaSource>,
    pub league: LeagueFactory,
    pub sink: UiSink,
    pub settings_path: Option<PathBuf>,
    pub updates: Option<UpdateConfig>,
    pub feedback: Option<FeedbackConfig>,
    pub hotkeys: Option<Box<dyn HotkeyBackend>>,
    pub free_space: Option<Arc<dyn FreeSpace>>,
}

impl std::fmt::Debug for EngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineConfig")
            .field("settings_path", &self.settings_path)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct Engine;

impl Engine {
    pub fn start(
        settings: Settings,
        media: Arc<dyn MediaSource>,
        league: LeagueFactory,
        sink: UiSink,
    ) -> EngineHandle {
        Self::start_with(EngineConfig {
            settings,
            media,
            league,
            sink,
            settings_path: Some(paths::settings_file()),
            updates: None,
            feedback: None,
            hotkeys: None,
            free_space: None,
        })
    }

    pub fn start_with(config: EngineConfig) -> EngineHandle {
        let EngineConfig {
            mut settings,
            media,
            league,
            sink,
            settings_path,
            updates,
            feedback,
            hotkeys,
            free_space,
        } = config;
        settings.validate();
        let (cmd, cmd_rx) = unbounded();
        let (league_tx, league_rx) = unbounded();
        let guard = league(league_tx);
        let thread = control::spawn(
            control::Spec {
                settings,
                media,
                settings_path,
                updates,
                feedback,
                hotkeys,
                free_space: free_space.unwrap_or_else(|| Arc::new(SystemFreeSpace)),
                cmd: cmd.clone(),
            },
            guard,
            league_rx,
            cmd_rx,
            crate::thumbs::wrap(Arc::from(sink)),
        );
        EngineHandle { cmd, thread }
    }
}

pub fn reveal(path: &Path) {
    let path = path.to_path_buf();
    let spawned = std::thread::Builder::new()
        .name("reveal".into())
        .spawn(move || reveal_now(&path));
    if let Err(err) = spawned {
        tracing::warn!(%err, "cannot start reveal thread");
    }
}

#[cfg(windows)]
fn reveal_now(path: &Path) {
    let mut command = std::process::Command::new("explorer.exe");
    if path.is_dir() {
        command.arg(path);
    } else {
        let mut select = std::ffi::OsString::from("/select,");
        select.push(path);
        command.arg(select);
    }
    run_viewer(command);
}

#[cfg(target_os = "macos")]
fn reveal_now(path: &Path) {
    let mut command = std::process::Command::new("open");
    if !path.is_dir() {
        command.arg("-R");
    }
    command.arg(path);
    run_viewer(command);
}

#[cfg(not(any(windows, target_os = "macos")))]
fn reveal_now(path: &Path) {
    tracing::debug!(path = %path.display(), "reveal is not supported on this platform");
}

#[cfg(any(windows, target_os = "macos"))]
fn run_viewer(mut command: std::process::Command) {
    // explorer.exe returns a nonzero exit code even when it succeeds
    if let Err(err) = command.status() {
        tracing::warn!(%err, "cannot reveal path");
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
