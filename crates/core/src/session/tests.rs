use super::*;
use crate::game::{GameEvent, Player, Team};

const SEC: Hns = HNS_PER_SEC;
const WINDOW: CaptureTarget = CaptureTarget::GameWindow { hwnd: 7, pid: 42 };

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

fn event(id: u32) -> TimedEvent {
    TimedEvent {
        id,
        game_secs: 10.0,
        at: 10 * SEC,
        kind: GameEvent::GameStart,
    }
}

fn found() -> Input {
    Input::WindowFound { hwnd: 7, pid: 42 }
}

fn recording_session(t0: Hns) -> Session {
    let mut s = Session::new(true);
    s.step(found());
    s.step(Input::Ready(info()));
    s.step(Input::MediaStarted { t0 });
    s.step(Input::Tick(t0));
    s
}

fn recording_status(since: Hns) -> Effect {
    Effect::Status(Status::Recording {
        since,
        game: Some(info()),
    })
}

#[test]
fn happy_path_records_tails_and_finalizes() {
    let mut s = Session::new(true);
    assert_eq!(s.step(found()), vec![Effect::Status(Status::Waiting)]);
    assert_eq!(
        s.step(Input::Ready(info())),
        vec![Effect::StartMedia(WINDOW)]
    );
    assert_eq!(
        s.step(Input::MediaStarted { t0: 100 }),
        vec![recording_status(100)]
    );
    assert_eq!(
        s.step(Input::Event(event(1))),
        vec![Effect::Highlight(event(1))]
    );
    s.step(Input::Tick(50 * SEC));
    assert!(s.step(Input::Ended(GameResult::Win)).is_empty());
    assert_eq!(
        s.step(Input::Tick(55 * SEC)),
        vec![
            Effect::StopMedia,
            Effect::Finalize {
                result: GameResult::Win
            },
            Effect::Status(Status::Finalizing),
        ]
    );
    assert_eq!(s.step(Input::Finalized), vec![Effect::Status(Status::Idle)]);
    assert_eq!(*s.state(), SessionState::Cooldown);
    s.step(found());
    assert_eq!(*s.state(), SessionState::Cooldown);
    s.step(Input::Gone);
    assert_eq!(*s.state(), SessionState::Idle);
}

#[test]
fn gone_before_ready_returns_to_idle_without_media() {
    let mut s = Session::new(true);
    s.step(found());
    assert_eq!(s.step(Input::Gone), vec![Effect::Status(Status::Idle)]);
    assert_eq!(*s.state(), SessionState::Idle);
}

#[test]
fn gone_while_starting_stops_media_and_goes_idle() {
    let mut s = Session::new(true);
    s.step(found());
    s.step(Input::Ready(info()));
    let effects = s.step(Input::Gone);
    assert!(effects.contains(&Effect::StopMedia));
    assert_eq!(*s.state(), SessionState::Idle);
    assert!(s.step(Input::MediaStarted { t0: 5 }).is_empty());
    assert_eq!(*s.state(), SessionState::Idle);
}

#[test]
fn skipped_game_enters_cooldown_until_window_is_gone() {
    let mut s = Session::new(true);
    s.step(found());
    s.step(Input::Skipped("TFT".into()));
    assert_eq!(*s.state(), SessionState::Cooldown);
    s.step(found());
    assert_eq!(*s.state(), SessionState::Cooldown);
    s.step(Input::Gone);
    assert_eq!(*s.state(), SessionState::Idle);
    assert!(!s.step(found()).is_empty());
}

#[test]
fn media_failure_while_starting_rearms_with_error() {
    let mut s = Session::new(true);
    s.step(found());
    s.step(Input::Ready(info()));
    assert_eq!(
        s.step(Input::MediaFailed("no gpu".into())),
        vec![Effect::Status(Status::Error("no gpu".into()))]
    );
    assert_eq!(*s.state(), SessionState::Armed { hwnd: 7, pid: 42 });
    assert_eq!(
        s.step(Input::Ready(info())),
        vec![Effect::StartMedia(WINDOW)]
    );
}

#[test]
fn media_failure_while_recording_finalizes() {
    let mut s = recording_session(0);
    let effects = s.step(Input::MediaFailed("device lost".into()));
    assert!(effects.contains(&Effect::Finalize {
        result: GameResult::Unknown
    }));
    assert_eq!(*s.state(), SessionState::Finalizing { manual: false });
}

#[test]
fn gone_while_recording_or_in_tail_finalizes() {
    let mut s = recording_session(0);
    let effects = s.step(Input::Gone);
    assert!(effects.contains(&Effect::Finalize {
        result: GameResult::Unknown
    }));

    let mut s = recording_session(0);
    s.step(Input::Ended(GameResult::Loss));
    let effects = s.step(Input::Gone);
    assert!(effects.contains(&Effect::Finalize {
        result: GameResult::Loss
    }));
    assert_eq!(*s.state(), SessionState::Finalizing { manual: false });
}

#[test]
fn tail_lasts_five_seconds_from_the_last_tick() {
    let mut s = recording_session(0);
    s.step(Input::Tick(100 * SEC));
    s.step(Input::Ended(GameResult::Win));
    assert_eq!(
        *s.state(),
        SessionState::Tail {
            until: 105 * SEC,
            result: GameResult::Win
        }
    );
    assert!(s.step(Input::Tick(105 * SEC - 1)).is_empty());
    assert!(matches!(s.state(), SessionState::Tail { .. }));
    assert!(!s.step(Input::Tick(105 * SEC)).is_empty());
    assert_eq!(*s.state(), SessionState::Finalizing { manual: false });
}

#[test]
fn events_forward_only_while_recording_or_in_tail() {
    let mut s = Session::new(true);
    assert!(s.step(Input::Event(event(1))).is_empty());
    s.step(found());
    s.step(Input::Ready(info()));
    assert!(s.step(Input::Event(event(2))).is_empty());
    s.step(Input::MediaStarted { t0: 0 });
    assert_eq!(s.step(Input::Event(event(3))).len(), 1);
    s.step(Input::Ended(GameResult::Win));
    assert_eq!(s.step(Input::Event(event(4))).len(), 1);
    s.step(Input::Tick(6 * SEC));
    assert!(s.step(Input::Event(event(5))).is_empty());
}

#[test]
fn three_hour_cap_stops_like_user_stop() {
    let mut s = recording_session(1000);
    assert!(s.step(Input::Tick(1000 + 3 * 3600 * SEC)).is_empty());
    let effects = s.step(Input::Tick(1000 + 3 * 3600 * SEC + 1));
    assert_eq!(
        effects,
        vec![
            Effect::StopMedia,
            Effect::Finalize {
                result: GameResult::Unknown
            },
            Effect::Status(Status::Finalizing),
        ]
    );
}

#[test]
fn manual_recording_starts_on_primary_monitor_and_returns_to_idle() {
    let mut s = Session::new(false);
    assert_eq!(
        s.step(Input::UserStart),
        vec![
            Effect::StartMedia(CaptureTarget::PrimaryMonitor),
            Effect::Status(Status::Waiting)
        ]
    );
    assert_eq!(
        s.step(Input::MediaStarted { t0: 9 }),
        vec![Effect::Status(Status::Recording {
            since: 9,
            game: None
        })]
    );
    s.step(Input::Gone);
    s.step(Input::Ended(GameResult::Win));
    assert!(matches!(s.state(), SessionState::Recording { .. }));
    let effects = s.step(Input::UserStop);
    assert!(effects.contains(&Effect::Finalize {
        result: GameResult::Unknown
    }));
    s.step(Input::Finalized);
    assert_eq!(*s.state(), SessionState::Idle);
}

#[test]
fn user_start_is_ignored_outside_idle() {
    let mut s = Session::new(true);
    s.step(found());
    assert!(s.step(Input::UserStart).is_empty());
    assert_eq!(*s.state(), SessionState::Armed { hwnd: 7, pid: 42 });
    let mut s = recording_session(0);
    assert!(s.step(Input::UserStart).is_empty());
}

#[test]
fn user_stop_in_game_recording_goes_to_cooldown_after_finalize() {
    let mut s = recording_session(0);
    s.step(Input::UserStop);
    s.step(Input::Finalized);
    assert_eq!(*s.state(), SessionState::Cooldown);
}

#[test]
fn auto_record_off_ignores_window_found_until_turned_on() {
    let mut s = Session::new(false);
    assert!(s.step(found()).is_empty());
    assert_eq!(*s.state(), SessionState::Idle);
    s.step(Input::SettingsChanged { auto_record: true });
    assert!(!s.step(found()).is_empty());
    assert_eq!(*s.state(), SessionState::Armed { hwnd: 7, pid: 42 });
}
