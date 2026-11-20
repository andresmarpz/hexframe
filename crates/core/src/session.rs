use crate::game::{GameInfo, GameResult, TimedEvent};
use crate::media::CaptureTarget;
use crate::time::{HNS_PER_SEC, Hns};

const TAIL: Hns = 5 * HNS_PER_SEC;
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
    },
    Starting {
        target: CaptureTarget,
        game: Option<GameInfo>,
    },
    Recording {
        since: Hns,
        game: Option<GameInfo>,
        manual: bool,
    },
    Tail {
        until: Hns,
        result: GameResult,
    },
    Finalizing {
        manual: bool,
    },
    Cooldown,
}

#[derive(Debug, Clone)]
pub struct Session {
    state: SessionState,
    now: Hns,
    auto_record: bool,
}

impl Session {
    pub fn new(auto_record: bool) -> Self {
        Self {
            state: SessionState::Idle,
            now: 0,
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
        self.state = next;
        effects
    }

    fn transition(&self, state: SessionState, input: Input) -> (SessionState, Vec<Effect>) {
        use SessionState::*;
        match (state, input) {
            (Idle, Input::WindowFound { hwnd, pid }) if self.auto_record => {
                (Armed { hwnd, pid }, vec![Effect::Status(Status::Waiting)])
            }
            (Idle, Input::UserStart) => {
                let target = CaptureTarget::PrimaryMonitor;
                (
                    Starting { target, game: None },
                    vec![Effect::StartMedia(target), Effect::Status(Status::Waiting)],
                )
            }
            (Armed { hwnd, pid }, Input::Ready(info)) => {
                let target = CaptureTarget::GameWindow { hwnd, pid };
                (
                    Starting {
                        target,
                        game: Some(info),
                    },
                    vec![Effect::StartMedia(target)],
                )
            }
            (Armed { .. }, Input::Gone) => (Idle, vec![Effect::Status(Status::Idle)]),
            (Starting { target, game }, Input::MediaStarted { t0 }) => {
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
            (Starting { target, .. }, Input::MediaFailed(reason)) => {
                let next = match target {
                    CaptureTarget::GameWindow { hwnd, pid } => Armed { hwnd, pid },
                    CaptureTarget::PrimaryMonitor => Idle,
                };
                (next, vec![Effect::Status(Status::Error(reason))])
            }
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
                },
                Vec::new(),
            ),
            (Recording { since, manual, .. }, Input::Tick(now)) if now - since > MAX_RECORDING => {
                finalize(GameResult::Unknown, manual)
            }
            (Recording { manual, .. }, Input::UserStop) => finalize(GameResult::Unknown, manual),
            (Recording { manual: false, .. }, Input::Gone) => finalize(GameResult::Unknown, false),
            (Recording { manual, .. }, Input::MediaFailed(_)) => {
                finalize(GameResult::Unknown, manual)
            }
            (state @ (Recording { .. } | Tail { .. }), Input::Event(event)) => {
                (state, vec![Effect::Highlight(event)])
            }
            (Tail { until, result }, Input::Tick(now)) if now >= until => finalize(result, false),
            (Tail { result, .. }, Input::Gone | Input::MediaFailed(_)) => finalize(result, false),
            (Finalizing { manual }, Input::Finalized) => {
                let next = if manual { Idle } else { Cooldown };
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

fn finalize(result: GameResult, manual: bool) -> (SessionState, Vec<Effect>) {
    (
        SessionState::Finalizing { manual },
        vec![
            Effect::StopMedia,
            Effect::Finalize { result },
            Effect::Status(Status::Finalizing),
        ],
    )
}

#[cfg(test)]
mod tests;
