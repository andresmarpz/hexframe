mod clips;
mod effects;
mod meta;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, never, select, tick, unbounded};
use greyscreen_core::keybinds::{Action, KeyCombo};
use greyscreen_core::media::{MediaEvent, MediaSession, MediaSource};
use greyscreen_core::session::{Input, Session, SessionState, Status};
use greyscreen_core::settings::Settings;
use greyscreen_core::time::{Hns, mono_now, secs_to_hns};
use greyscreen_league::LeagueMsg;

use crate::api::{EngineCmd, LeagueGuard, NoticeLevel, UiMsg};
use crate::clipjob::ClipResult;
use crate::delete;
use crate::hotkey::{self, HotkeyBackend};
use crate::library;
use crate::settings_save::SettingsSaver;
use crate::storage::ActiveFolder;
use crate::updater::{self, UpdateCmd, UpdateConfig, UpdateHandle};
use crate::writer::Report;
use meta::Recording;

const TICK: Duration = Duration::from_millis(250);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(15);

pub(crate) type Sink = Arc<dyn Fn(UiMsg) + Send + Sync>;

pub(crate) enum Internal {
    MediaStopped(u64),
    WriterDone(Report),
    ClipDone(ClipResult),
}

pub(crate) struct Spec {
    pub(crate) settings: Settings,
    pub(crate) media: Arc<dyn MediaSource>,
    pub(crate) settings_path: Option<PathBuf>,
    pub(crate) updates: Option<UpdateConfig>,
    pub(crate) hotkeys: Option<Box<dyn HotkeyBackend>>,
    pub(crate) cmd: Sender<EngineCmd>,
}

#[derive(Debug)]
struct StartedMedia {
    video: greyscreen_core::media::VideoFormat,
    audio: Option<greyscreen_core::media::AudioFormat>,
    encoder: String,
}

struct Control {
    settings: Settings,
    session: Session,
    media: Arc<dyn MediaSource>,
    sink: Sink,
    media_rx: Receiver<MediaEvent>,
    media_session: Option<Box<dyn MediaSession>>,
    media_gen: u64,
    stopping: bool,
    started: Option<StartedMedia>,
    recording: Option<Recording>,
    orphan_finalize: bool,
    internal_tx: Sender<Internal>,
    internal_rx: Receiver<Internal>,
    shutting_down: bool,
    test_stop_at: Option<Hns>,
    settings_saver: SettingsSaver,
    updater: Option<UpdateHandle>,
    update_busy: bool,
    hotkeys: Box<dyn HotkeyBackend>,
    applied_hotkeys: Vec<(Action, KeyCombo)>,
    active_folder: ActiveFolder,
}

pub(crate) fn spawn(
    spec: Spec,
    guard: LeagueGuard,
    league_rx: Receiver<LeagueMsg>,
    cmd_rx: Receiver<EngineCmd>,
    sink: Sink,
) -> Option<JoinHandle<()>> {
    let spawned = thread::Builder::new()
        .name("engine-control".into())
        .spawn(move || {
            let panic_sink = sink.clone();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                Control::new(spec, sink).run(cmd_rx, league_rx);
            }));
            if outcome.is_err() {
                tracing::error!("engine control thread panicked");
                panic_sink(UiMsg::Notice {
                    level: NoticeLevel::Error,
                    text: "The recording engine crashed. Restart Greyscreen.".to_owned(),
                });
                panic_sink(UiMsg::Status(Status::Error("engine crashed".to_owned())));
            }
            drop(guard);
        });
    match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::error!(%err, "cannot start engine control thread");
            None
        }
    }
}

impl Control {
    fn new(spec: Spec, sink: Sink) -> Self {
        let Spec {
            settings,
            media,
            settings_path,
            updates,
            hotkeys,
            cmd,
        } = spec;
        let (internal_tx, internal_rx) = unbounded();
        let updater =
            updates.and_then(|config| updater::spawn(config, settings.check_updates, sink.clone()));
        Self {
            updater,
            update_busy: false,
            settings_saver: SettingsSaver::spawn(settings_path, sink.clone()),
            hotkeys: hotkeys.unwrap_or_else(|| hotkey::platform(cmd, sink.clone())),
            applied_hotkeys: settings.keymap().global(),
            active_folder: ActiveFolder::default(),
            test_stop_at: None,
            session: Session::new(settings.auto_record),
            settings,
            media,
            sink,
            media_rx: never(),
            media_session: None,
            media_gen: 0,
            stopping: false,
            started: None,
            recording: None,
            orphan_finalize: false,
            internal_tx,
            internal_rx,
            shutting_down: false,
        }
    }

    fn run(mut self, mut cmd_rx: Receiver<EngineCmd>, mut league_rx: Receiver<LeagueMsg>) {
        let ticker = tick(TICK);
        self.hotkeys.set(&self.applied_hotkeys);
        library::spawn_startup(
            self.settings.output_dir.clone(),
            self.settings.storage_limit_bytes(),
            self.active_folder.clone(),
            self.sink.clone(),
        );
        let mut deadline: Option<Instant> = None;
        loop {
            let media_rx = self.media_rx.clone();
            let internal_rx = self.internal_rx.clone();
            select! {
                recv(cmd_rx) -> cmd => match cmd {
                    Ok(cmd) => self.on_cmd(cmd),
                    Err(_) => {
                        cmd_rx = never();
                        self.shutting_down = true;
                    }
                },
                recv(league_rx) -> msg => match msg {
                    Ok(msg) => self.on_league(msg),
                    Err(_) => league_rx = never(),
                },
                recv(media_rx) -> event => match event {
                    Ok(event) => self.on_media(event),
                    Err(_) => self.media_rx = never(),
                },
                recv(internal_rx) -> msg => {
                    if let Ok(msg) = msg {
                        self.on_internal(msg);
                    }
                },
                recv(ticker) -> _ => self.on_tick(),
            }
            self.sync_update_busy();
            if self.shutting_down {
                let deadline = *deadline.get_or_insert_with(|| Instant::now() + SHUTDOWN_GRACE);
                self.stop_active();
                if self.idle() || Instant::now() >= deadline {
                    break;
                }
            }
        }
    }

    fn on_cmd(&mut self, cmd: EngineCmd) {
        match cmd {
            EngineCmd::Shutdown => self.shutting_down = true,
            EngineCmd::StopRecording => self.feed(Input::UserStop),
            EngineCmd::Retry => self.feed(Input::Retry),
            EngineCmd::ApplySettings(settings) => self.apply_settings(settings),
            EngineCmd::StartTestRecording { secs } => self.start_test_recording(secs),
            EngineCmd::RescanLibrary => {
                library::spawn_scan(self.settings.output_dir.clone(), self.sink.clone());
            }
            EngineCmd::DeleteRecording { id } => self.delete_recording(id),
            EngineCmd::SaveClipNow => self.save_clip_now(),
            EngineCmd::CheckForUpdates => self.update_cmd(UpdateCmd::Check),
            EngineCmd::InstallUpdate => self.update_cmd(UpdateCmd::Install),
            EngineCmd::GlobalAction(action) => self.global_action(action),
        }
    }

    fn global_action(&mut self, action: Action) {
        if action == Action::SaveClip
            && matches!(self.session.state(), SessionState::Recording { .. })
        {
            self.save_clip_now();
        }
    }

    fn delete_recording(&self, id: String) {
        let active = self
            .recording
            .as_ref()
            .and_then(|recording| recording.folder.file_name())
            .map(|name| name.to_string_lossy().into_owned());
        delete::spawn(
            self.settings.output_dir.clone(),
            id,
            active,
            self.sink.clone(),
        );
    }

    fn apply_settings(&mut self, mut settings: Settings) {
        settings.validate();
        let auto_record = settings.auto_record;
        self.update_cmd_quiet(UpdateCmd::Auto(settings.check_updates));
        self.settings_saver.save(settings.clone());
        let global = settings.keymap().global();
        if global != self.applied_hotkeys {
            self.applied_hotkeys = global;
            self.hotkeys.set(&self.applied_hotkeys);
        }
        self.settings = settings;
        self.feed(Input::SettingsChanged { auto_record });
    }

    fn on_league(&mut self, msg: LeagueMsg) {
        let input = match msg {
            LeagueMsg::WindowFound { hwnd, pid } => Input::WindowFound { hwnd, pid },
            LeagueMsg::Ready(info) => Input::Ready(info),
            LeagueMsg::Event(event) => Input::Event(event),
            LeagueMsg::Ended(result) => Input::Ended(result),
            LeagueMsg::Gone => Input::Gone,
            LeagueMsg::Skipped(reason) => Input::Skipped(reason),
        };
        self.feed(input);
    }

    fn on_tick(&mut self) {
        let now = mono_now();
        self.feed(Input::Tick(now));
        self.stop_test_recording(now);
        self.run_due_clips(now);
        if let Some(recording) = self.recording.as_mut()
            && recording.sidecar_due(now)
            && let Err(err) = recording.write_sidecar(now)
        {
            tracing::warn!(%err, "cannot write recording sidecar");
        }
    }

    fn start_test_recording(&mut self, secs: u32) {
        self.feed(Input::UserStart);
        if matches!(self.session.state(), SessionState::Starting { .. }) {
            self.test_stop_at = Some(mono_now() + secs_to_hns(f64::from(secs)));
        }
    }

    fn stop_test_recording(&mut self, now: Hns) {
        let Some(stop_at) = self.test_stop_at else {
            return;
        };
        if now < stop_at {
            return;
        }
        match self.session.state() {
            SessionState::Starting { .. } => {}
            SessionState::Recording { manual: true, .. } => {
                self.test_stop_at = None;
                self.feed(Input::UserStop);
            }
            _ => self.test_stop_at = None,
        }
    }

    fn stop_active(&mut self) {
        match self.session.state() {
            SessionState::Recording { .. } => self.feed(Input::UserStop),
            SessionState::Tail { .. }
            | SessionState::Starting {
                target: greyscreen_core::media::CaptureTarget::GameWindow { .. },
                ..
            } => self.feed(Input::Gone),
            _ => {}
        }
    }

    fn idle(&self) -> bool {
        matches!(
            self.session.state(),
            SessionState::Idle | SessionState::Armed { .. } | SessionState::Cooldown
        ) && self.recording.is_none()
            && !self.stopping
    }

    fn feed(&mut self, input: Input) {
        let effects = self.session.step(input);
        self.run_effects(effects);
        self.sync_update_busy();
    }

    fn update_busy_now(&self) -> bool {
        !matches!(
            self.session.state(),
            SessionState::Idle | SessionState::Cooldown
        ) || self.recording.is_some()
            || self.stopping
    }

    fn sync_update_busy(&mut self) {
        let busy = self.update_busy_now();
        if busy != self.update_busy {
            self.update_busy = busy;
            self.update_cmd_quiet(UpdateCmd::Busy(busy));
        }
    }

    fn update_cmd_quiet(&self, cmd: UpdateCmd) {
        if let Some(updater) = &self.updater {
            updater.send(cmd);
        }
    }

    fn update_cmd(&self, cmd: UpdateCmd) {
        match &self.updater {
            Some(updater) => updater.send(cmd),
            None => self.send(UiMsg::Update(greyscreen_core::update::UpdateState::Failed(
                "Updates are not available in this build.".into(),
            ))),
        }
    }

    fn send(&self, msg: UiMsg) {
        (self.sink)(msg);
    }

    fn notice(&self, level: NoticeLevel, text: impl Into<String>) {
        self.send(UiMsg::Notice {
            level,
            text: text.into(),
        });
    }
}
