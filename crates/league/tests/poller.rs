mod common;

use std::time::Duration;

use common::{FakeServer, Switch, expect_silence, recv};
use crossbeam_channel::unbounded;
use greyscreen_core::game::{GameEvent, GameResult, Team};
use greyscreen_core::time::{HNS_PER_SEC, mono_now};
use greyscreen_league::{LeagueMsg, LiveClient, Poller};
use serde_json::json;

const FAST: Duration = Duration::from_millis(20);

fn spawn(
    server: &FakeServer,
    switch: &Switch,
) -> (
    greyscreen_league::PollerHandle,
    crossbeam_channel::Receiver<LeagueMsg>,
) {
    let (tx, rx) = unbounded();
    let handle = Poller::spawn_with_interval(
        LiveClient::new(&server.base),
        Box::new(switch.clone()),
        tx,
        FAST,
    );
    (handle, rx)
}

#[test]
fn full_match_ready_events_end_gone() {
    let server = FakeServer::start();
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);

    assert_eq!(
        recv(&rx),
        LeagueMsg::WindowFound {
            hwnd: 42,
            pid: 4242
        }
    );
    expect_silence(&rx, 150);

    server.with(|s| {
        s.push_event_now(json!({"EventName":"GameStart"}));
        s.push_event_now(json!({"EventName":"MinionsSpawning"}));
        s.game_time_base = 5000.0;
    });
    let ready_start = mono_now();
    server.with(|s| s.ready = true);

    let LeagueMsg::Ready(info) = recv(&rx) else {
        panic!("expected Ready");
    };
    assert_eq!(info.me.name, "Faker");
    assert_eq!(info.me.champion, "Ahri");
    assert_eq!(info.me.team, Team::Order);
    assert_eq!(info.players.len(), 2);
    assert_eq!(info.mode, "CLASSIC");
    expect_silence(&rx, 150);

    let mut ids = Vec::new();
    let mut pushed_at = Vec::new();
    for _ in 0..3 {
        let before = mono_now();
        ids.push(server.with(|s| {
            s.push_event_now(json!({
                "EventName":"ChampionKill","KillerName":"Faker#KR1","VictimName":"Chovy","Assisters":[]
            }))
        }));
        pushed_at.push(before);
        std::thread::sleep(Duration::from_millis(60));
    }
    let end_id = server.with(|s| s.push_event_now(json!({"EventName":"GameEnd","Result":"Win"})));
    let end_pushed = mono_now();

    let mut last_at = i64::MIN;
    for (id, before) in ids.iter().zip(&pushed_at) {
        let LeagueMsg::Event(event) = recv(&rx) else {
            panic!("expected Event");
        };
        assert_eq!(event.id, *id);
        assert_eq!(
            event.kind,
            GameEvent::ChampionKill {
                killer: "Faker".into(),
                victim: "Chovy".into(),
                assisters: vec![]
            }
        );
        assert!(event.at >= last_at, "at must not go backwards");
        last_at = event.at;
        let slack = HNS_PER_SEC;
        assert!(
            event.at >= before - slack && event.at <= mono_now() + slack,
            "event time {} not near push time {}",
            event.at,
            before
        );
        assert!(event.at > ready_start);
    }
    let LeagueMsg::Event(end) = recv(&rx) else {
        panic!("expected GameEnd event");
    };
    assert_eq!(end.id, end_id);
    assert!(end.at >= last_at);
    assert!(end.at <= end_pushed + HNS_PER_SEC);
    assert_eq!(
        end.kind,
        GameEvent::GameEnd {
            result: GameResult::Win
        }
    );
    assert_eq!(recv(&rx), LeagueMsg::Ended(GameResult::Win));
    expect_silence(&rx, 150);

    switch.set(false);
    assert_eq!(recv(&rx), LeagueMsg::Gone);
    expect_silence(&rx, 100);

    switch.set(true);
    assert_eq!(
        recv(&rx),
        LeagueMsg::WindowFound {
            hwnd: 42,
            pid: 4242
        }
    );
}

#[test]
fn history_before_ready_is_dropped_and_events_are_not_repeated() {
    let server = FakeServer::start();
    server.with(|s| {
        s.ready = true;
        s.push_event_now(json!({"EventName":"GameStart"}));
        s.push_event_now(json!({"EventName":"MinionsSpawning"}));
        s.game_time_base = 900.0;
    });
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);

    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    assert!(matches!(recv(&rx), LeagueMsg::Ready(_)));
    expect_silence(&rx, 200);

    server.with(|s| s.push_event_now(json!({"EventName":"FirstBlood","Recipient":"Faker"})));
    let LeagueMsg::Event(event) = recv(&rx) else {
        panic!("expected Event");
    };
    assert_eq!(event.id, 2);
    assert_eq!(
        event.kind,
        GameEvent::FirstBlood {
            recipient: "Faker".into()
        }
    );
    expect_silence(&rx, 200);
}

#[test]
fn tft_and_replay_are_skipped_until_the_window_goes() {
    for (mode, replay) in [("TFT", false), ("CLASSIC", true)] {
        let server = FakeServer::start();
        server.with(|s| {
            s.ready = true;
            s.mode = mode.into();
            s.replay = replay;
        });
        let switch = Switch::on();
        let (_handle, rx) = spawn(&server, &switch);

        assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
        assert!(matches!(recv(&rx), LeagueMsg::Skipped(_)));
        expect_silence(&rx, 150);
        switch.set(false);
        assert_eq!(recv(&rx), LeagueMsg::Gone);
    }
}

#[test]
fn spectator_without_active_player_is_skipped() {
    let server = FakeServer::start();
    server.with(|s| {
        s.ready = true;
        s.active_player = None;
    });
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    assert!(matches!(recv(&rx), LeagueMsg::Skipped(_)));
}

#[test]
fn refused_polls_after_ready_emit_gone_once() {
    let server = FakeServer::start();
    server.with(|s| s.ready = true);
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    assert!(matches!(recv(&rx), LeagueMsg::Ready(_)));

    server.shutdown();
    assert_eq!(recv(&rx), LeagueMsg::Gone);
    expect_silence(&rx, 200);
    switch.set(false);
    expect_silence(&rx, 200);
}

#[test]
fn timeouts_count_as_not_ready_and_never_emit_gone() {
    let server = FakeServer::start();
    server.with(|s| s.ready = true);
    let switch = Switch::on();
    let (tx, rx) = unbounded();
    let client = LiveClient::with_timeouts(
        &server.base,
        Duration::from_millis(50),
        Duration::from_millis(50),
    );
    let _handle = Poller::spawn_with_interval(client, Box::new(switch.clone()), tx, FAST);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    assert!(matches!(recv(&rx), LeagueMsg::Ready(_)));

    server.with(|s| s.stall = Duration::from_millis(150));
    expect_silence(&rx, 800);
}

#[test]
fn window_lost_while_waiting_emits_gone() {
    let server = FakeServer::start();
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    switch.set(false);
    assert_eq!(recv(&rx), LeagueMsg::Gone);
}

struct Counting(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl greyscreen_league::locator::GameLocator for Counting {
    fn find(&mut self) -> Option<(isize, u32)> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        None
    }
}

#[test]
fn a_missing_game_is_searched_for_at_most_once_per_interval() {
    let server = FakeServer::start();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (tx, _rx) = unbounded();
    let handle = Poller::spawn_with_interval(
        LiveClient::new(&server.base),
        Box::new(Counting(calls.clone())),
        tx,
        Duration::from_millis(100),
    );
    std::thread::sleep(Duration::from_millis(1050));
    drop(handle);
    let searches = calls.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        (5..=11).contains(&searches),
        "searched {searches} times in 1.05 s"
    );
}

#[test]
fn dropping_the_handle_stops_the_thread() {
    let server = FakeServer::start();
    let switch = Switch::on();
    let (handle, rx) = spawn(&server, &switch);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    drop(handle);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            _ => assert!(
                std::time::Instant::now() < deadline,
                "poller thread kept running after the handle was dropped"
            ),
        }
    }
}

#[test]
fn a_sixty_second_interval_poller_joins_in_under_a_second() {
    let server = FakeServer::start();
    let switch = Switch::on();
    let (tx, rx) = unbounded();
    let handle = Poller::spawn_with_interval(
        LiveClient::new(&server.base),
        Box::new(switch.clone()),
        tx,
        Duration::from_secs(60),
    );
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    let started = std::time::Instant::now();
    drop(handle);
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "stop took {:?}",
        started.elapsed()
    );
}

fn secs(hns: i64) -> f64 {
    hns as f64 / HNS_PER_SEC as f64
}

fn next_event(rx: &crossbeam_channel::Receiver<LeagueMsg>) -> greyscreen_core::game::TimedEvent {
    loop {
        if let LeagueMsg::Event(event) = recv(rx) {
            return event;
        }
    }
}

fn kill() -> serde_json::Value {
    json!({"EventName":"ChampionKill","KillerName":"Faker#KR1","VictimName":"Chovy","Assisters":[]})
}

#[test]
fn a_frozen_loading_clock_does_not_shift_later_events() {
    let server = FakeServer::start();
    server.with(|s| {
        s.ready = true;
        s.game_time_base = 0.0;
        s.freeze();
        s.push_event_now(json!({"EventName":"GameStart"}));
    });
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    assert!(matches!(recv(&rx), LeagueMsg::Ready(_)));
    let start = next_event(&rx);
    assert_eq!(start.kind, GameEvent::GameStart);

    std::thread::sleep(Duration::from_millis(400));
    server.with(|s| s.unfreeze());
    let mut pushed = Vec::new();
    for _ in 0..3 {
        pushed.push(mono_now());
        server.with(|s| s.push_event_now(kill()));
        std::thread::sleep(Duration::from_millis(150));
    }
    for at in pushed {
        let event = next_event(&rx);
        assert!(
            (secs(event.at - at)).abs() < 0.25,
            "event landed {:.2} s from where it happened",
            secs(event.at - at)
        );
    }
}

#[test]
fn a_paused_game_clock_keeps_events_on_the_wall_clock() {
    let server = FakeServer::start();
    server.with(|s| {
        s.ready = true;
        s.game_time_base = 20.0;
    });
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    assert!(matches!(recv(&rx), LeagueMsg::Ready(_)));
    expect_silence(&rx, 100);

    let first = mono_now();
    server.with(|s| s.push_event_now(kill()));
    let first_event = next_event(&rx);
    server.with(|s| s.freeze());
    std::thread::sleep(Duration::from_millis(500));
    server.with(|s| s.unfreeze());
    let second = mono_now();
    server.with(|s| s.push_event_now(kill()));
    let second_event = next_event(&rx);

    assert!(secs(first_event.at - first).abs() < 0.25);
    assert!(secs(second_event.at - second).abs() < 0.25);
    assert!(secs(second_event.at - first_event.at) > 0.45);
}

#[test]
fn events_that_arrive_in_one_batch_keep_their_own_times() {
    let server = FakeServer::start();
    server.with(|s| s.ready = true);
    let switch = Switch::on();
    let (_handle, rx) = spawn(&server, &switch);
    assert!(matches!(recv(&rx), LeagueMsg::WindowFound { .. }));
    assert!(matches!(recv(&rx), LeagueMsg::Ready(_)));
    expect_silence(&rx, 100);

    server.with(|s| s.stall = Duration::from_millis(600));
    std::thread::sleep(Duration::from_millis(100));
    let mut pushed = Vec::new();
    for _ in 0..3 {
        pushed.push(mono_now());
        server.with(|s| s.push_event_now(kill()));
        std::thread::sleep(Duration::from_millis(200));
    }
    server.with(|s| s.stall = Duration::ZERO);
    for at in pushed {
        let event = next_event(&rx);
        assert!(
            secs(event.at - at).abs() < 0.3,
            "event landed {:.2} s from where it happened",
            secs(event.at - at)
        );
    }
}
