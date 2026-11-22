use super::*;
use crate::game::{GameEvent, Player, Team};
use crate::problem::NO_FRAMES;
use proptest::prelude::*;

const SEC: Hns = HNS_PER_SEC;
const TIMING: SessionTiming = SessionTiming {
    start_timeout: 15 * SEC,
    retry_delay: 5 * SEC,
};

fn info() -> GameInfo {
    let me = Player {
        name: "Me".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    };
    GameInfo {
        mode: "CLASSIC".into(),
        map: Some(11),
        me: me.clone(),
        players: vec![me],
    }
}

fn game_starts(effects: &[Effect]) -> usize {
    effects
        .iter()
        .filter(|effect| matches!(effect, Effect::StartMedia(CaptureTarget::GameWindow { .. })))
        .count()
}

fn finalizes(effects: &[Effect]) -> usize {
    effects
        .iter()
        .filter(|effect| matches!(effect, Effect::Finalize { .. }))
        .count()
}

#[derive(Debug, Clone)]
enum Op {
    Found(u32),
    Ready,
    Event,
    Ended,
    Gone,
    Skipped,
    Started,
    Failed(Fault),
    Finalized,
    UserStart,
    UserStop,
    Retry,
    Settings(bool),
    Tick(Hns),
}

#[derive(Debug, Clone, Copy)]
enum Fault {
    NoFrames,
    Unsupported,
    Setup,
}

impl Fault {
    fn reason(self) -> &'static str {
        match self {
            Fault::NoFrames => NO_FRAMES,
            Fault::Unsupported => "graphics capture is not supported",
            Fault::Setup => "access is denied",
        }
    }
}

fn fault() -> impl Strategy<Value = Fault> {
    prop_oneof![
        Just(Fault::NoFrames),
        Just(Fault::Unsupported),
        Just(Fault::Setup)
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        2 => (40u32..44).prop_map(Op::Found),
        2 => Just(Op::Ready),
        1 => Just(Op::Event),
        1 => Just(Op::Ended),
        1 => Just(Op::Gone),
        1 => Just(Op::Skipped),
        2 => Just(Op::Started),
        2 => fault().prop_map(Op::Failed),
        2 => Just(Op::Finalized),
        1 => Just(Op::UserStart),
        2 => Just(Op::UserStop),
        1 => Just(Op::Retry),
        1 => any::<bool>().prop_map(Op::Settings),
        4 => (0i64..30_000).prop_map(|ms| Op::Tick(ms * SEC / 1000)),
    ]
}

struct Driver {
    session: Session,
    now: Hns,
}

impl Driver {
    fn new(auto_record: bool) -> Self {
        let mut driver = Self {
            session: Session::with_timing(auto_record, TIMING),
            now: SEC,
        };
        driver.session.step(Input::Tick(SEC));
        driver
    }

    fn apply(&mut self, op: &Op) -> Vec<Effect> {
        let input = match op {
            Op::Found(pid) => Input::WindowFound { hwnd: 7, pid: *pid },
            Op::Ready => Input::Ready(info()),
            Op::Event => Input::Event(TimedEvent {
                id: 1,
                game_secs: 1.0,
                at: self.now,
                kind: GameEvent::GameStart,
            }),
            Op::Ended => Input::Ended(GameResult::Win),
            Op::Gone => Input::Gone,
            Op::Skipped => Input::Skipped("not league".into()),
            Op::Started => Input::MediaStarted { t0: self.now },
            Op::Failed(fault) => Input::MediaFailed(fault.reason().into()),
            Op::Finalized => Input::Finalized,
            Op::UserStart => Input::UserStart,
            Op::UserStop => Input::UserStop,
            Op::Retry => Input::Retry,
            Op::Settings(auto_record) => Input::SettingsChanged {
                auto_record: *auto_record,
            },
            Op::Tick(dt) => {
                self.now += dt;
                Input::Tick(self.now)
            }
        };
        self.session.step(input)
    }
}

proptest! {
    #[test]
    fn lifecycle_never_sticks_or_double_stops(
        auto_record in any::<bool>(),
        ops in proptest::collection::vec(op(), 1..120),
    ) {
        let mut driver = Driver::new(auto_record);
        let mut auto = auto_record;
        let mut recordings = 0usize;
        let mut finalized_count = 0usize;
        let mut pending_finalize = false;
        for op in &ops {
            let was_recording = matches!(driver.session.state(), SessionState::Recording { .. });
            let effects = driver.apply(op);
            if let Op::Settings(value) = op {
                auto = *value;
            }
            let state = driver.session.state().clone();
            if !was_recording && matches!(state, SessionState::Recording { .. }) {
                recordings += 1;
            }
            finalized_count += finalizes(&effects);
            prop_assert!(finalized_count <= recordings, "finalise without a recording");
            if finalizes(&effects) > 0 {
                pending_finalize = true;
            }
            if matches!(op, Op::Finalized) {
                pending_finalize = false;
            }
            if matches!(state, SessionState::Finalizing { .. }) {
                prop_assert!(pending_finalize, "finalising without a pending finalise");
                let again = driver.session.clone().step(Input::UserStop);
                prop_assert!(again.is_empty(), "a second stop acts: {again:?}");
            }
            if matches!(op, Op::Finalized) {
                prop_assert!(!matches!(state, SessionState::Finalizing { .. }), "stuck finalising");
            }
            if matches!(op, Op::Tick(_))
                && let SessionState::Starting { started, .. } = &state
            {
                prop_assert!(
                    started.is_some_and(|at| driver.now - at < TIMING.start_timeout),
                    "start outlived its timeout"
                );
            }
            if !auto {
                prop_assert!(game_starts(&effects) == 0, "automatic start with auto record off");
                prop_assert!(
                    !matches!(state, SessionState::Armed { retry: Some(_), .. }),
                    "retry pending with auto record off"
                );
            }
        }
    }

    #[test]
    fn stop_while_idle_or_stopping_is_inert(extra_stops in 1usize..5) {
        let mut driver = Driver::new(true);
        prop_assert!(driver.apply(&Op::UserStop).is_empty());
        driver.apply(&Op::Found(42));
        driver.apply(&Op::Ready);
        driver.apply(&Op::Started);
        let first = driver.apply(&Op::UserStop);
        prop_assert_eq!(finalizes(&first), 1);
        for _ in 0..extra_stops {
            prop_assert!(driver.apply(&Op::UserStop).is_empty());
        }
    }
}

#[derive(Debug, Clone)]
enum Outcome {
    Fail(Fault),
    Silent,
}

fn outcome() -> impl Strategy<Value = Outcome> {
    prop_oneof![fault().prop_map(Outcome::Fail), Just(Outcome::Silent)]
}

struct Run {
    driver: Driver,
    starts: Vec<Hns>,
    outcome: Outcome,
}

impl Run {
    fn new(outcome: Outcome) -> Self {
        let mut driver = Driver::new(true);
        driver.apply(&Op::Found(42));
        let effects = driver.apply(&Op::Ready);
        let mut run = Self {
            driver,
            starts: Vec::new(),
            outcome,
        };
        run.react(&effects);
        run
    }

    fn react(&mut self, effects: &[Effect]) {
        let mut pending = game_starts(effects);
        while pending > 0 {
            pending -= 1;
            self.starts.push(self.driver.now);
            if let Outcome::Fail(fault) = self.outcome {
                let more = self.driver.apply(&Op::Failed(fault));
                pending += game_starts(&more);
            }
        }
    }

    fn tick(&mut self, dt: Hns) -> Vec<Effect> {
        let effects = self.driver.apply(&Op::Tick(dt));
        self.react(&effects);
        effects
    }
}

proptest! {
    #[test]
    fn automatic_starts_are_spaced_and_capped(
        outcome in outcome(),
        dts in proptest::collection::vec(1i64..12_000, 1..400),
    ) {
        let mut run = Run::new(outcome);
        for dt in dts {
            run.tick(dt * SEC / 1000);
        }
        for pair in run.starts.windows(2) {
            prop_assert!(pair[1] - pair[0] >= TIMING.retry_delay, "starts {pair:?} too close");
        }
        prop_assert!(run.starts.len() <= 1 + MAX_START_RETRIES as usize);
    }

    #[test]
    fn unsupported_is_never_retried(dts in proptest::collection::vec(1i64..20_000, 1..100)) {
        let mut run = Run::new(Outcome::Fail(Fault::Unsupported));
        for dt in dts {
            run.tick(dt * SEC / 1000);
        }
        prop_assert_eq!(run.starts.len(), 1);
    }

    #[test]
    fn starts_per_simulated_hour_stay_bounded(
        outcome in outcome(),
        step_ms in 1i64..5_000,
    ) {
        let mut run = Run::new(outcome);
        let end = SEC + 3600 * SEC;
        while run.driver.now < end {
            run.tick(step_ms * SEC / 1000);
        }
        prop_assert!(run.starts.len() <= 1 + MAX_START_RETRIES as usize);
    }

    #[test]
    fn user_retry_and_new_target_start_at_once(
        fault in prop_oneof![Just(Fault::NoFrames), Just(Fault::Setup)],
        new_pid in 100u32..200,
    ) {
        let mut run = Run::new(Outcome::Fail(fault));
        for _ in 0..40 {
            run.tick(SEC);
        }
        prop_assert_eq!(run.starts.len(), 1 + MAX_START_RETRIES as usize);
        let before = run.starts.len();
        let retried = run.driver.apply(&Op::Retry);
        prop_assert_eq!(game_starts(&retried), 1);
        run.react(&retried);
        prop_assert!(run.starts.len() > before);

        let mut run = Run::new(Outcome::Fail(Fault::Unsupported));
        run.tick(SEC);
        let retried = run.driver.apply(&Op::Retry);
        prop_assert_eq!(game_starts(&retried), 1);

        let mut run = Run::new(Outcome::Fail(Fault::Unsupported));
        run.driver.apply(&Op::Gone);
        run.driver.apply(&Op::Found(new_pid));
        let ready = run.driver.apply(&Op::Ready);
        prop_assert_eq!(game_starts(&ready), 1);
        let SessionState::Starting { retries, .. } = run.driver.session.state() else {
            return Err(TestCaseError::fail("not starting"));
        };
        prop_assert_eq!(*retries, 0);
    }

    #[test]
    fn silent_start_fails_exactly_at_the_timeout(early in 0i64..15_000) {
        let mut run = Run::new(Outcome::Silent);
        let almost = early * SEC / 1000;
        run.tick(almost);
        prop_assert!(
            matches!(run.driver.session.state(), SessionState::Starting { .. }),
            "failed early"
        );
        let effects = run.tick(TIMING.start_timeout - almost);
        prop_assert!(effects.contains(&Effect::StopMedia));
        prop_assert!(
            matches!(
                run.driver.session.state(),
                SessionState::Armed { retry: Some(_), .. }
            ),
            "no retry queued"
        );
    }
}
