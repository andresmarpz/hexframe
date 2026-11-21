use crate::game::{GameInfo, GameResult, TimedEvent};
use crate::media::CaptureTarget;
use crate::time::{HNS_PER_SEC, Hns};

const TAIL: Hns = 5 * HNS_PER_SEC;
const START_TIMEOUT: Hns = 15 * HNS_PER_SEC;
const START_RETRY_DELAY: Hns = 5 * HNS_PER_SEC;
const MAX_START_RETRIES: u32 = 5;
const MAX_RECORDING: Hns = 3 * 3600 * HNS_PER_SEC;

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

#[derive(Debug, Clone)]
pub struct Session {
    state: SessionState,
    now: Hns,
    last_pid: Option<u32>,
    auto_record: bool,
}

impl Session {
    pub fn new(auto_record: bool) -> Self {
        Self {
            state: SessionState::Idle,
            now: 0,
            last_pid: None,
            auto_record,
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    pub fn step(&mut self, input: Input) -> Vec<Effect> {
        match input {
            Input::Tick(now) => self.now = now,
            Input::SettingsChanged { auto_record } => {
                self.auto_record = auto_record;
                return Vec::new();
            }
            _ => {}
        }
        let state = std::mem::replace(&mut self.state, SessionState::Idle);
        let (next, effects) = self.transition(state, input);
        if let SessionState::Armed { pid, .. } = &next {
            self.last_pid = Some(*pid);
        }
        self.state = next;
        effects
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
            (Armed { .. }, Input::Gone) => (Idle, vec![Effect::Status(Status::Idle)]),
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
            ) if now - started >= START_TIMEOUT => self.start_failed(
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
            ) => (Idle, vec![Effect::StopMedia, Effect::Status(Status::Idle)]),
            (
                Starting {
                    target: CaptureTarget::GameWindow { .. },
                    ..
                },
                Input::Skipped(_),
            ) => (
                Cooldown,
                vec![Effect::StopMedia, Effect::Status(Status::Idle)],
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
                (next, vec![Effect::Status(Status::Idle)])
            }
            (Cooldown, Input::Gone) => (Idle, Vec::new()),
            (Idle | Armed { .. } | Cooldown, Input::Skipped(_)) => {
                (Cooldown, vec![Effect::Status(Status::Idle)])
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
        match game {
            Some(game) if retries < MAX_START_RETRIES => (
                SessionState::Armed {
                    hwnd,
                    pid,
                    retry: Some(StartRetry {
                        game,
                        retries: retries + 1,
                        at: self.now + START_RETRY_DELAY,
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
mod tests;
