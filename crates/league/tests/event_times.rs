mod common;

use common::{FakeServer, Switch, recv};
use crossbeam_channel::unbounded;
use greyscreen_core::game::TimedEvent;
use greyscreen_core::time::HNS_PER_SEC;
use greyscreen_league::{LeagueMsg, LiveClient, Poller};
use serde_json::json;
use std::time::Duration;

const READY_AT: i64 = 10 * HNS_PER_SEC;
const BEFORE_STATS: i64 = 20 * HNS_PER_SEC;
const AFTER_STATS: i64 = 24 * HNS_PER_SEC;
const MID_STATS: i64 = 22 * HNS_PER_SEC;

fn fake_clock() -> greyscreen_league::Clock {
    let mut calls = 0_u32;
    Box::new(move || {
        calls += 1;
        match calls {
            1 => READY_AT,
            n if n % 2 == 0 => BEFORE_STATS,
            _ => AFTER_STATS,
        }
    })
}

fn kill_at(id: u32, game_secs: f64) -> serde_json::Value {
    json!({
        "EventName":"ChampionKill","KillerName":"Faker#KR1","VictimName":"Chovy","Assisters":[],
        "EventID": id, "EventTime": game_secs
    })
}

fn collect(rx: &crossbeam_channel::Receiver<LeagueMsg>, count: usize) -> Vec<TimedEvent> {
    let mut events = Vec::new();
    while events.len() < count {
        if let LeagueMsg::Event(event) = recv(rx) {
            events.push(event);
        }
    }
    events
}

#[test]
fn an_event_lands_at_the_poll_midpoint_minus_the_game_clock_plus_its_own_time() {
    let server = FakeServer::start();
    server.with(|s| {
        s.ready = true;
        s.freeze_at(300.0);
        s.events = vec![
            kill_at(0, 287.999_999_9),
            kill_at(1, 288.0),
            kill_at(2, 300.0),
            kill_at(3, 310.0),
        ];
    });
    let switch = Switch::on();
    let (tx, rx) = unbounded();
    let _handle = Poller::spawn_with_clock(
        LiveClient::new(&server.base),
        Box::new(switch),
        tx,
        Duration::from_millis(20),
        fake_clock(),
    );

    let events = collect(&rx, 3);

    let at: Vec<i64> = events.iter().map(|e| e.at).collect();
    assert_eq!(
        at,
        vec![READY_AT, MID_STATS, MID_STATS + 10 * HNS_PER_SEC],
        "an event exactly at ready time is kept, one a tick earlier is dropped"
    );
    let ids: Vec<u32> = events.iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![1, 2, 3]);
}
