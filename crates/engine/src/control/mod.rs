mod effects;
mod meta;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, never, select, tick, unbounded};
use greyscreen_core::media::{MediaEvent, MediaSession, MediaSource};
use greyscreen_core::session::{Input, Session, SessionState, Status};
use greyscreen_core::settings::Settings;
use greyscreen_core::time::mono_now;
use greyscreen_league::LeagueMsg;

use crate::api::{EngineCmd, LeagueGuard, NoticeLevel, UiMsg};
use crate::writer::Report;
use meta::Recording;

const TICK: Duration = Duration::from_millis(250);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(15);

pub(crate) type Sink = Arc<dyn Fn(UiMsg) + Send + Sync>;

pub(crate) enum Internal {
    MediaStopped(u64),
    WriterDone(Report),
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
}

pub(crate) fn spawn(
    settings: Settings,
    media: Arc<dyn MediaSource>,
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
                Control::new(settings, media, sink).run(cmd_rx, league_rx);
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
    fn new(settings: Settings, media: Arc<dyn MediaSource>, sink: Sink) -> Self {
        let (internal_tx, internal_rx) = unbounded();
        Self {
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
            EngineCmd::ApplySettings(settings) => self.apply_settings(settings),
            EngineCmd::StartTestRecording { .. }
            | EngineCmd::RescanLibrary
            | EngineCmd::SaveClipNow => tracing::debug!(?cmd, "command not handled yet"),
        }
    }

    fn apply_settings(&mut self, mut settings: Settings) {
        settings.validate();
        let auto_record = settings.auto_record;
        let keep = effects::ring_keep(&settings);
        self.settings = settings;
        if let Some(recording) = self.recording.as_mut() {
            recording.ring.set_keep(keep);
        }
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
        if let Some(recording) = self.recording.as_mut()
            && recording.sidecar_due(now)
            && let Err(err) = recording.write_sidecar(now)
        {
            tracing::warn!(%err, "cannot write recording sidecar");
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
