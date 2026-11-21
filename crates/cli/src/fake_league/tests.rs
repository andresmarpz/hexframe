use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use greyscreen_core::game::{GameEvent, GameResult};
use greyscreen_league::{AlwaysPresent, LeagueMsg, LiveClient, Poller};

use super::{FakeLeague, Script};

#[test]
fn builtin_script_parses_and_counts_involvement() {
    let script = Script::builtin().unwrap();
    assert_eq!(script.players.len(), 10);
    assert_eq!(script.active_player, "Me#EUW");
    let (mut kills, mut deaths, mut assists) = (0, 0, 0);
    for event in &script.events {
        if event["EventName"] != "ChampionKill" {
            continue;
        }
        if event["KillerName"] == "Me" {
            kills += 1;
        }
        if event["VictimName"] == "Me" {
            deaths += 1;
        }
        if event["Assisters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n == "Me")
        {
            assists += 1;
        }
    }
    assert_eq!((kills, deaths, assists), (2, 2, 2));
}

#[test]
fn gamestats_is_404_while_loading_then_200() {
    let mut script = Script::builtin().unwrap();
    script.loading_secs = 0.6;
    let league = FakeLeague::start(0, 1.0, script).unwrap();
    let client = LiveClient::new(&format!("http://127.0.0.1:{}", league.port()));
    assert!(client.gamestats().is_err());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(stats) = client.gamestats() {
            assert!(stats.game_time.is_some());
            break;
        }
        assert!(Instant::now() < deadline, "gamestats never became ready");
        std::thread::sleep(Duration::from_millis(30));
    }
    assert_eq!(client.activeplayername().unwrap(), "Me#EUW");
    assert_eq!(client.playerlist().unwrap().len(), 10);
    assert!(!client.replay_playback_ok());
}

#[test]
fn poller_sees_ready_events_and_win() {
    let league = FakeLeague::start(0, 60.0, Script::builtin().unwrap()).unwrap();
    let client = LiveClient::new(&format!("http://127.0.0.1:{}", league.port()));
    let (tx, rx) = unbounded();
    let _handle = Poller::spawn_with_interval(
        client,
        Box::new(AlwaysPresent),
        tx,
        Duration::from_millis(40),
    );
    let deadline = Instant::now() + Duration::from_secs(90);
    let (mut ready, mut events, mut result) = (false, 0, None);
    while result.is_none() {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left).expect("poller went silent") {
            LeagueMsg::Ready(info) => {
                assert_eq!(info.me.name, "Me");
                assert_eq!(info.me.champion, "Ahri");
                assert_eq!(info.players.len(), 10);
                ready = true;
            }
            LeagueMsg::Event(event) => {
                assert!(ready, "event before Ready");
                if !matches!(event.kind, GameEvent::GameEnd { .. }) {
                    events += 1;
                }
            }
            LeagueMsg::Ended(r) => result = Some(r),
            _ => {}
        }
    }
    assert!(events >= 10, "only {events} events");
    assert_eq!(result, Some(GameResult::Win));
}
