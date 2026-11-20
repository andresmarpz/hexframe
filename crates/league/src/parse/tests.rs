use serde_json::json;

use super::*;

fn parsed(value: serde_json::Value) -> GameEvent {
    to_game_event(&value)
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn simple_events() {
    assert_eq!(
        parsed(json!({"EventID":0,"EventName":"GameStart","EventTime":0.03})),
        GameEvent::GameStart
    );
    assert_eq!(
        parsed(json!({"EventID":1,"EventName":"MinionsSpawning","EventTime":65.0})),
        GameEvent::MinionsSpawning
    );
    assert_eq!(
        parsed(json!({"EventName":"FirstBlood","Recipient":"Faker"})),
        GameEvent::FirstBlood {
            recipient: "Faker".into()
        }
    );
}

#[test]
fn champion_kill_with_assisters() {
    let event = parsed(json!({
        "EventName":"ChampionKill","KillerName":"Faker","VictimName":"Chovy","Assisters":["Keria"]
    }));
    assert_eq!(
        event,
        GameEvent::ChampionKill {
            killer: "Faker".into(),
            victim: "Chovy".into(),
            assisters: names(&["Keria"]),
        }
    );
}

#[test]
fn missing_assisters_is_empty() {
    let event = parsed(json!({"EventName":"BaronKill","KillerName":"Faker","Stolen":"False"}));
    assert_eq!(
        event,
        GameEvent::BaronKill {
            stolen: false,
            killer: "Faker".into(),
            assisters: vec![]
        }
    );
}

#[test]
fn multikill_ace_and_first_brick() {
    assert_eq!(
        parsed(json!({"EventName":"Multikill","KillerName":"Faker","KillStreak":2})),
        GameEvent::Multikill {
            killer: "Faker".into(),
            streak: 2
        }
    );
    assert_eq!(
        parsed(json!({"EventName":"Ace","Acer":"Faker","AcingTeam":"ORDER"})),
        GameEvent::Ace {
            acer: "Faker".into(),
            acing_team: Team::Order
        }
    );
    assert_eq!(
        parsed(json!({"EventName":"FirstBrick","KillerName":"Faker"})),
        GameEvent::FirstBrick {
            killer: "Faker".into()
        }
    );
}

#[test]
fn structures() {
    assert_eq!(
        parsed(json!({
            "EventName":"TurretKilled","TurretKilled":"Turret_T2_L_03_A","KillerName":"Faker","Assisters":["Keria"]
        })),
        GameEvent::TurretKilled {
            turret: "Turret_T2_L_03_A".into(),
            killer: "Faker".into(),
            assisters: names(&["Keria"]),
        }
    );
    assert_eq!(
        parsed(json!({
            "EventName":"InhibKilled","InhibKilled":"Barracks_T2_R1","KillerName":"Faker","Assisters":[]
        })),
        GameEvent::InhibKilled {
            inhib: "Barracks_T2_R1".into(),
            killer: "Faker".into(),
            assisters: vec![],
        }
    );
    assert_eq!(
        parsed(json!({"EventName":"InhibRespawningSoon","InhibRespawningSoon":"Barracks_T2_R1"})),
        GameEvent::InhibRespawningSoon {
            inhib: "Barracks_T2_R1".into()
        }
    );
    assert_eq!(
        parsed(json!({"EventName":"InhibRespawned","InhibRespawned":"Barracks_T2_R1"})),
        GameEvent::InhibRespawned {
            inhib: "Barracks_T2_R1".into()
        }
    );
}

#[test]
fn objectives_accept_string_and_bool_stolen() {
    let dragon = |stolen: serde_json::Value| {
        parsed(json!({
            "EventName":"DragonKill","DragonType":"Earth","Stolen":stolen,"KillerName":"Faker","Assisters":[]
        }))
    };
    for (input, expected) in [
        (json!("True"), true),
        (json!("true"), true),
        (json!(true), true),
        (json!("False"), false),
        (json!(false), false),
    ] {
        assert_eq!(
            dragon(input),
            GameEvent::DragonKill {
                dragon_type: "Earth".into(),
                stolen: expected,
                killer: "Faker".into(),
                assisters: vec![],
            }
        );
    }
    assert_eq!(
        parsed(json!({"EventName":"HeraldKill","Stolen":"True","KillerName":"Faker"})),
        GameEvent::HeraldKill {
            stolen: true,
            killer: "Faker".into(),
            assisters: vec![]
        }
    );
    assert_eq!(
        parsed(json!({"EventName":"HordeKill","KillerName":"Faker","Assisters":["Keria"]})),
        GameEvent::HordeKill {
            killer: "Faker".into(),
            assisters: names(&["Keria"])
        }
    );
}

#[test]
fn game_end_results() {
    let end = |result: &str| parsed(json!({"EventName":"GameEnd","Result":result}));
    assert_eq!(
        end("Win"),
        GameEvent::GameEnd {
            result: GameResult::Win
        }
    );
    assert_eq!(
        end("Lose"),
        GameEvent::GameEnd {
            result: GameResult::Loss
        }
    );
    assert_eq!(
        end("???"),
        GameEvent::GameEnd {
            result: GameResult::Unknown
        }
    );
}

#[test]
fn unknown_and_malformed_events_do_not_panic() {
    assert_eq!(
        parsed(json!({"EventName":"AtakhanKill"})),
        GameEvent::Unknown {
            name: "AtakhanKill".into()
        }
    );
    assert_eq!(
        parsed(json!({"EventName":"Ace","Acer":"Faker"})),
        GameEvent::Unknown { name: "Ace".into() }
    );
    assert_eq!(parsed(json!(42)), GameEvent::Unknown { name: "".into() });
}

#[test]
fn tags_are_stripped_from_event_names() {
    let event = parsed(json!({
        "EventName":"ChampionKill","KillerName":"Faker#KR1","VictimName":"Chovy","Assisters":["Keria#T1"]
    }));
    assert_eq!(
        event,
        GameEvent::ChampionKill {
            killer: "Faker".into(),
            victim: "Chovy".into(),
            assisters: names(&["Keria"]),
        }
    );
}

fn entry(game_name: &str, tag: &str, champion: &str, team: &str, bot: bool) -> PlayerEntry {
    PlayerEntry {
        champion_name: champion.into(),
        is_bot: bot,
        summoner_name: game_name.into(),
        riot_id: if tag.is_empty() {
            String::new()
        } else {
            format!("{game_name}#{tag}")
        },
        riot_id_game_name: if tag.is_empty() {
            String::new()
        } else {
            game_name.into()
        },
        riot_id_tag_line: tag.into(),
        team: team.into(),
        ..PlayerEntry::default()
    }
}

fn stats() -> GameStats {
    GameStats {
        game_mode: "CLASSIC".into(),
        game_time: Some(12.0),
        map_number: Some(11),
        ..GameStats::default()
    }
}

#[test]
fn game_info_finds_me_by_riot_id() {
    let players = vec![
        entry("Faker", "KR1", "Ahri", "ORDER", false),
        entry("Faker", "EUW", "Zed", "CHAOS", false),
        entry("Chovy", "KR2", "Lux", "CHAOS", false),
    ];
    let info = to_game_info("Faker#EUW", &players, &stats()).unwrap();
    assert_eq!(info.me.champion, "Zed");
    assert_eq!(info.me.team, Team::Chaos);
    assert_eq!(info.me.name, "Faker");
    assert_eq!(info.mode, "CLASSIC");
    assert_eq!(info.map, Some(11));
    assert_eq!(info.players.len(), 3);
}

#[test]
fn game_info_without_tag_falls_back_to_summoner_name() {
    let players = vec![
        entry("Bot Ahri", "", "Ahri", "ORDER", true),
        entry("Faker", "", "Lux", "CHAOS", false),
    ];
    let info = to_game_info("faker", &players, &stats()).unwrap();
    assert_eq!(info.me.champion, "Lux");
    assert_eq!(info.me.name, "Faker");
    assert!(info.players.iter().any(|p| p.is_bot));
}

#[test]
fn game_info_is_none_when_me_is_missing() {
    let players = vec![entry("Chovy", "KR2", "Lux", "CHAOS", false)];
    assert!(to_game_info("Faker#KR1", &players, &stats()).is_none());
    assert!(to_game_info("Faker#KR1", &[], &stats()).is_none());
}

#[test]
fn player_list_deserializes_riot_fields() {
    let players: Vec<PlayerEntry> = serde_json::from_value(json!([{
        "championName":"Ahri","isBot":false,"summonerName":"Faker","riotId":"Faker#KR1",
        "riotIdGameName":"Faker","riotIdTagLine":"KR1","team":"ORDER","unknownField":1
    }]))
    .unwrap();
    assert_eq!(players[0].riot_id, "Faker#KR1");
    assert_eq!(players[0].riot_id_tag_line, "KR1");
    assert_eq!(players[0].champion_name, "Ahri");
}
