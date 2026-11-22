use crate::game::{GameInfo, GameResult, TimedEvent};
use crate::media::CaptureTarget;
use crate::problem::Problem;
use crate::time::{HNS_PER_SEC, Hns};

mod timing;
pub use timing::SessionTiming;

const TAIL: Hns = 5 * HNS_PER_SEC;
const MAX_START_RETRIES: u32 = 5;
pub const MAX_RECORDING: Hns = 3 * 3600 * HNS_PER_SEC;

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    WindowFound { hwnd: isize, pid: u32 },
    Ready(GameInfo),
    Event(TimedEvent),
    Ended(GameResult),
    Gone,
    Skipped(String),
    MediaStarted { t0: Hns },
    MediaFailed(String),
    Finalized,
    UserStart,
    UserStop,
    Retry,
    Tick(Hns),
    SettingsChanged { auto_record: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    StartMedia(CaptureTarget),
    StopMedia,
    Finalize { result: GameResult },
    Highlight(TimedEvent),
    Status(Status),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Idle,
    GameRunning,
    Waiting,
    Recording { since: Hns, game: Option<GameInfo> },
    Finalizing,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Idle,
    Armed {
        hwnd: isize,
        pid: u32,
        retry: Option<StartRetry>,
    },
    Starting {
        target: CaptureTarget,
        game: Option<GameInfo>,
        retries: u32,
        started: Option<Hns>,
    },
    Recording {
        since: Hns,
        game: Option<GameInfo>,
        manual: bool,
    },
    Tail {
        until: Hns,
        result: GameResult,
        gone: bool,
    },
    Finalizing {
        manual: bool,
        gone: bool,
    },
    Cooldown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartRetry {
    game: GameInfo,
    retries: u32,
    at: Hns,
}

#[derive(Debug, Clone, PartialEq)]
struct LiveGame {
    hwnd: isize,
    pid: u32,
    info: Option<GameInfo>,
    ended: bool,
}

#[derive(Debug, Clone)]
pub struct Session {
    state: SessionState,
    now: Hns,
    last_pid: Option<u32>,
    auto_record: bool,
    live: Option<LiveGame>,
    timing: SessionTiming,
}

impl Session {
    pub fn new(auto_record: bool) -> Self {
        Self::with_timing(auto_record, SessionTiming::default())
    }

    pub fn with_timing(auto_record: bool, timing: SessionTiming) -> Self {
        Self {
            state: SessionState::Idle,
            now: 0,
            last_pid: None,
            auto_record,
            live: None,
            timing,
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    pub fn step(&mut self, input: Input) -> Vec<Effect> {
        match &input {
            Input::Tick(now) => self.now = *now,
            Input::SettingsChanged { auto_record } => return self.set_auto_record(*auto_record),
            Input::Retry => return self.retry(),
            _ => {}
        }
        self.track_live(&input);
        let finalized = matches!(input, Input::Finalized);
        let state = std::mem::replace(&mut self.state, SessionState::Idle);
        let (next, mut effects) = self.transition(state, input);
        if let SessionState::Armed { pid, .. } = &next {
            self.last_pid = Some(*pid);
        }
        self.state = next;
        if finalized && let Some(resumed) = self.resume() {
            effects.extend(resumed);
        }
        effects
    }

    fn track_live(&mut self, input: &Input) {
        match input {
            Input::WindowFound { hwnd, pid } => {
                self.live = Some(LiveGame {
                    hwnd: *hwnd,
                    pid: *pid,
                    info: None,
                    ended: false,
                });
            }
            Input::Ready(info) => {
                if let Some(live) = &mut self.live {
                    live.info = Some(info.clone());
                }
            }
            Input::Ended(_) => {
                if let Some(live) = &mut self.live {
                    live.ended = true;
                }
            }
            Input::Gone | Input::Skipped(_) => self.live = None,
            _ => {}
        }
    }

    fn game_running(&self) -> bool {
        self.live.as_ref().is_some_and(|live| !live.ended)
    }

    fn idle_status(&self) -> Status {
        if !self.auto_record && self.game_running() {
            Status::GameRunning
        } else {
            Status::Idle
        }
    }

    fn set_auto_record(&mut self, auto_record: bool) -> Vec<Effect> {
        if self.auto_record == auto_record {
            return Vec::new();
        }
        self.auto_record = auto_record;
        if auto_record {
            return self.resume().unwrap_or_default();
        }
        let state = std::mem::replace(&mut self.state, SessionState::Idle);
        let (next, effects) = self.stop_for_switch(state);
        self.state = next;
        effects
    }

    fn retry(&mut self) -> Vec<Effect> {
        if matches!(self.state, SessionState::Cooldown) {
            self.state = SessionState::Idle;
        }
        if let Some(resumed) = self.resume() {
            return resumed;
        }
        match self.state {
            SessionState::Armed { .. } => vec![Effect::Status(Status::Waiting)],
            SessionState::Idle => vec![Effect::Status(self.idle_status())],
            _ => Vec::new(),
        }
    }

    fn stop_for_switch(&self, state: SessionState) -> (SessionState, Vec<Effect>) {
        use SessionState::*;
        match state {
            Armed { .. } => (Idle, vec![Effect::Status(self.idle_status())]),
            Starting {
                target: CaptureTarget::GameWindow { .. },
                ..
            } => (
                Idle,
                vec![Effect::StopMedia, Effect::Status(self.idle_status())],
            ),
            Recording { manual: false, .. } => finalize(GameResult::Unknown, true, false),
            Tail { result, .. } => finalize(result, true, false),
            other => (other, Vec::new()),
        }
    }

    fn resume(&mut self) -> Option<Vec<Effect>> {
        if !self.auto_record || !matches!(self.state, SessionState::Idle) {
            return None;
        }
        let live = self.live.as_ref().filter(|live| !live.ended)?;
        let (hwnd, pid) = (live.hwnd, live.pid);
        self.last_pid = Some(pid);
        let Some(info) = live.info.clone() else {
            self.state = armed(hwnd, pid);
            return Some(vec![Effect::Status(Status::Waiting)]);
        };
        let target = CaptureTarget::GameWindow { hwnd, pid };
        self.state = self.starting(target, Some(info), 0);
        Some(vec![
            Effect::StartMedia(target),
            Effect::Status(Status::Waiting),
        ])
    }

    fn transition(&self, state: SessionState, input: Input) -> (SessionState, Vec<Effect>) {
        use SessionState::*;
        match (state, input) {
            (Idle, Input::WindowFound { hwnd, pid }) if self.auto_record => {
                (armed(hwnd, pid), vec![Effect::Status(Status::Waiting)])
            }
            (Cooldown, Input::WindowFound { hwnd, pid })
                if self.auto_record && self.last_pid.is_some_and(|last| last != pid) =>
            {
                (armed(hwnd, pid), vec![Effect::Status(Status::Waiting)])
            }
            (Idle, Input::WindowFound { .. }) => (Idle, vec![Effect::Status(Status::GameRunning)]),
            (Idle, Input::Ended(_)) if !self.auto_record => {
                (Idle, vec![Effect::Status(self.idle_status())])
            }
            (Idle, Input::UserStart) => {
                let target = CaptureTarget::PrimaryMonitor;
                (
                    self.starting(target, None, 0),
                    vec![Effect::StartMedia(target), Effect::Status(Status::Waiting)],
                )
            }
            (Armed { hwnd, pid, .. }, Input::Ready(info)) => {
                let target = CaptureTarget::GameWindow { hwnd, pid };
                (
                    self.starting(target, Some(info), 0),
                    vec![Effect::StartMedia(target)],
                )
            }
            (
                Armed {
                    hwnd,
                    pid,
                    retry: Some(retry),
                },
                Input::Tick(now),
            ) if now >= retry.at => {
                let target = CaptureTarget::GameWindow { hwnd, pid };
                (
                    self.starting(target, Some(retry.game), retry.retries),
                    vec![Effect::StartMedia(target)],
                )
            }
            (Armed { .. }, Input::Gone) => (Idle, vec![Effect::Status(self.idle_status())]),
            (Starting { target, game, .. }, Input::MediaStarted { t0 }) => {
                let status = Status::Recording {
                    since: t0,
                    game: game.clone(),
                };
                (
                    Recording {
                        since: t0,
                        game,
                        manual: target == CaptureTarget::PrimaryMonitor,
                    },
                    vec![Effect::Status(status)],
                )
            }
            (
                Starting {
                    target,
                    game,
                    retries,
                    ..
                },
                Input::MediaFailed(reason),
            ) => self.start_failed(target, game, retries, reason, false),
            (
                Starting {
                    target,
                    game,
                    retries,
                    started: Some(started),
                },
                Input::Tick(now),
            ) if now - started >= self.timing.start_timeout => self.start_failed(
                target,
                game,
                retries,
                "capture did not start".to_owned(),
                true,
            ),
            (
                Starting {
                    target,
                    game,
                    retries,
                    started: None,
                },
                Input::Tick(now),
            ) => (
                Starting {
                    target,
                    game,
                    retries,
                    started: Some(now),
                },
                Vec::new(),
            ),
            (
                Starting {
                    target: CaptureTarget::GameWindow { .. },
                    ..
                },
                Input::Gone,
            ) => (
                Idle,
                vec![Effect::StopMedia, Effect::Status(self.idle_status())],
            ),
            (
                Starting {
                    target: CaptureTarget::GameWindow { .. },
                    ..
                },
                Input::Skipped(_),
            ) => (
                Cooldown,
                vec![Effect::StopMedia, Effect::Status(self.idle_status())],
            ),
            (Recording { manual: false, .. }, Input::Ended(result)) => (
                Tail {
                    until: self.now + TAIL,
                    result,
                    gone: false,
                },
                Vec::new(),
            ),
            (Recording { since, manual, .. }, Input::Tick(now)) if now - since > MAX_RECORDING => {
                finalize(GameResult::Unknown, manual, false)
            }
            (Recording { manual, .. }, Input::UserStop) => {
                finalize(GameResult::Unknown, manual, false)
            }
            (Recording { manual: false, .. }, Input::Gone) => {
                finalize(GameResult::Unknown, false, true)
            }
            (Recording { manual, .. }, Input::MediaFailed(_)) => {
                finalize(GameResult::Unknown, manual, false)
            }
            (state @ (Recording { .. } | Tail { .. }), Input::Event(event)) => {
                (state, vec![Effect::Highlight(event)])
            }
            (
                Tail {
                    until,
                    result,
                    gone,
                },
                Input::Tick(now),
            ) if now >= until => finalize(result, false, gone),
            (Tail { result, .. }, Input::Gone) => finalize(result, false, true),
            (Tail { result, gone, .. }, Input::MediaFailed(_)) => finalize(result, false, gone),
            (Finalizing { manual, .. }, Input::Gone) => {
                (Finalizing { manual, gone: true }, Vec::new())
            }
            (Finalizing { manual, gone }, Input::Finalized) => {
                let next = if manual || gone { Idle } else { Cooldown };
                (next, vec![Effect::Status(self.idle_status())])
            }
            (Cooldown, Input::Gone) => (Idle, Vec::new()),
            (Idle | Armed { .. } | Cooldown, Input::Skipped(_)) => {
                (Cooldown, vec![Effect::Status(self.idle_status())])
            }
            (state, _) => (state, Vec::new()),
        }
    }
}

impl Session {
    fn starting(
        &self,
        target: CaptureTarget,
        game: Option<GameInfo>,
        retries: u32,
    ) -> SessionState {
        SessionState::Starting {
            target,
            game,
            retries,
            started: (self.now > 0).then_some(self.now),
        }
    }

    fn start_failed(
        &self,
        target: CaptureTarget,
        game: Option<GameInfo>,
        retries: u32,
        reason: String,
        stop_media: bool,
    ) -> (SessionState, Vec<Effect>) {
        let mut effects = Vec::new();
        if stop_media {
            effects.push(Effect::StopMedia);
        }
        let CaptureTarget::GameWindow { hwnd, pid } = target else {
            effects.push(Effect::Status(Status::Error(reason)));
            return (SessionState::Idle, effects);
        };
        let retry_helps = Problem::from_error(&reason).retry_helps();
        match game {
            Some(game) if retries < MAX_START_RETRIES && retry_helps => (
                SessionState::Armed {
                    hwnd,
                    pid,
                    retry: Some(StartRetry {
                        game,
                        retries: retries + 1,
                        at: self.now + self.timing.retry_delay,
                    }),
                },
                effects,
            ),
            Some(_) => {
                effects.push(Effect::Status(Status::Error(reason)));
                (SessionState::Cooldown, effects)
            }
            None => {
                effects.push(Effect::Status(Status::Error(reason)));
                (armed(hwnd, pid), effects)
            }
        }
    }
}

fn armed(hwnd: isize, pid: u32) -> SessionState {
    SessionState::Armed {
        hwnd,
        pid,
        retry: None,
    }
}

fn finalize(result: GameResult, manual: bool, gone: bool) -> (SessionState, Vec<Effect>) {
    (
        SessionState::Finalizing { manual, gone },
        vec![
            Effect::StopMedia,
            Effect::Finalize { result },
            Effect::Status(Status::Finalizing),
        ],
    )
}

#[cfg(test)]
mod properties;
#[cfg(test)]
mod tests;
