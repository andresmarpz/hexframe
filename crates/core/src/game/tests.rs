use super::*;

#[test]
fn timed_event_json_round_trip() {
    let event = TimedEvent {
        id: 7,
        game_secs: 612.5,
        at: 123_456_789,
        kind: GameEvent::ChampionKill {
            killer: "Ahri".into(),
            victim: "Zed".into(),
            assisters: vec!["Lux".into()],
        },
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains(r#""type":"champion_kill""#));
    let back: TimedEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back, event);
}

#[test]
fn unit_variant_and_team_round_trip() {
    let json = serde_json::to_string(&GameEvent::GameStart).unwrap();
    assert_eq!(json, r#"{"type":"game_start"}"#);
    let ace = GameEvent::Ace {
        acer: "Ahri".into(),
        acing_team: Team::Chaos,
    };
    let json = serde_json::to_string(&ace).unwrap();
    assert!(json.contains(r#""acing_team":"chaos""#));
    assert_eq!(serde_json::from_str::<GameEvent>(&json).unwrap(), ace);
}

fn player(name: &str, champion: &str, team: Team) -> Player {
    Player {
        name: name.into(),
        champion: champion.into(),
        team,
        is_bot: false,
    }
}

fn roster(others: &[Player]) -> GameInfo {
    let me = player("Dotd", "Volibear", Team::Order);
    let mut players = vec![me.clone()];
    players.extend_from_slice(others);
    GameInfo {
        mode: "KIWI".into(),
        map: Some(12),
        me,
        players,
    }
}

#[test]
fn me_answers_to_game_name_riot_id_and_unique_champion() {
    let info = roster(&[player("Foe", "Zed", Team::Chaos)]);
    assert!(info.is_me("Dotd"));
    assert!(info.is_me("dotd#EUW"));
    assert!(info.is_me("Volibear"));
    assert!(!info.is_me("Zed"));
    assert!(!info.is_me("Foe"));
}

#[test]
fn me_does_not_answer_to_a_champion_that_two_players_share() {
    let info = roster(&[player("Foe", "Volibear", Team::Chaos)]);
    assert!(!info.is_me("Volibear"));
    assert!(info.is_me("Dotd"));
}

#[test]
fn a_player_with_my_champion_as_a_name_is_not_me() {
    let info = roster(&[player("Volibear", "Zed", Team::Chaos)]);
    assert!(!info.is_me("Volibear"));
}

#[test]
fn players_are_found_by_name_or_unique_champion() {
    let info = roster(&[player("Foe", "Zed", Team::Chaos)]);
    assert_eq!(info.player_named("foe#EUW").unwrap().champion, "Zed");
    assert_eq!(info.player_named("Zed").unwrap().name, "Foe");
    assert_eq!(info.player_named("Volibear").unwrap().name, "Dotd");
    assert!(info.player_named("Minion_T200L0S3").is_none());
    let twins = roster(&[player("Foe", "Volibear", Team::Chaos)]);
    assert!(twins.player_named("Volibear").is_none());
}
