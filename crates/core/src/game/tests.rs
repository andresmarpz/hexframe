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
