use super::*;
use crate::game::{GameResult, Player};

fn player(name: &str, champion: &str, team: Team) -> Player {
    Player {
        name: name.into(),
        champion: champion.into(),
        team,
        is_bot: false,
    }
}

fn info() -> GameInfo {
    let me = player("Me#EUW", "Ahri", Team::Order);
    GameInfo {
        mode: "ARAM".into(),
        map: Some(12),
        players: vec![
            me.clone(),
            player("Ally#EUW", "Jinx", Team::Order),
            player("Foe#EUW", "Zed", Team::Chaos),
            player("Foe2#EUW", "Lux", Team::Chaos),
        ],
        me,
    }
}

fn kill(killer: &str, victim: &str, assisters: &[&str]) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.into(),
        victim: victim.into(),
        assisters: assisters.iter().map(|name| (*name).to_owned()).collect(),
    }
}

fn category(kind: GameEvent) -> EventCategory {
    category_of(&kind, &info())
}

#[test]
fn kills_involving_me_are_mine() {
    assert_eq!(category(kill("Foe", "Me", &[])), EventCategory::MyDeaths);
    assert_eq!(
        category(kill("Foe", "Me#EUW", &["Lux"])),
        EventCategory::MyDeaths
    );
    assert_eq!(category(kill("Me", "Foe", &[])), EventCategory::MyKills);
    assert_eq!(
        category(kill("Ally", "Foe", &["Me"])),
        EventCategory::MyAssists
    );
}

#[test]
fn kills_between_other_players_split_by_the_killers_team() {
    assert_eq!(
        category(kill("Ally", "Foe", &[])),
        EventCategory::TeammateKills
    );
    assert_eq!(
        category(kill("Foe", "Ally", &["Foe2"])),
        EventCategory::EnemyKills
    );
}

#[test]
fn an_executed_champion_counts_by_the_victims_team() {
    assert_eq!(
        category(kill("Turret_T2_L_03_A", "Ally", &[])),
        EventCategory::EnemyKills
    );
    assert_eq!(
        category(kill("Minion_T100L0S1", "Foe", &[])),
        EventCategory::TeammateKills
    );
    assert_eq!(
        category(kill("SRU_Baron", "Stranger", &[])),
        EventCategory::EnemyKills
    );
}

#[test]
fn champion_named_events_split_the_same_way() {
    assert_eq!(category(kill("Zed", "Ahri", &[])), EventCategory::MyDeaths);
    assert_eq!(category(kill("Ahri", "Zed", &[])), EventCategory::MyKills);
    assert_eq!(
        category(kill("Jinx", "Zed", &["Ahri"])),
        EventCategory::MyAssists
    );
    assert_eq!(
        category(kill("Jinx", "Zed", &[])),
        EventCategory::TeammateKills
    );
    assert_eq!(
        category(kill("Lux", "Jinx", &[])),
        EventCategory::EnemyKills
    );
}

#[test]
fn a_champion_two_players_share_is_not_me() {
    let mut game = info();
    game.players.push(player("Twin#EUW", "Ahri", Team::Chaos));
    assert_ne!(
        category_of(&kill("Zed", "Ahri", &[]), &game),
        EventCategory::MyDeaths
    );
}

#[test]
fn multikills_split_into_mine_and_others() {
    let multi = |killer: &str| GameEvent::Multikill {
        killer: killer.into(),
        streak: 3,
    };
    assert_eq!(category(multi("Me")), EventCategory::MyMultikills);
    assert_eq!(category(multi("Foe")), EventCategory::OtherMultikills);
    assert_eq!(category(multi("Ally")), EventCategory::OtherMultikills);
}

#[test]
fn objectives_and_structures_have_their_own_categories() {
    let by = |killer: &str| (killer.to_owned(), Vec::<String>::new());
    let (killer, assisters) = by("Foe");
    assert_eq!(
        category(GameEvent::DragonKill {
            dragon_type: "Fire".into(),
            stolen: false,
            killer: killer.clone(),
            assisters: assisters.clone(),
        }),
        EventCategory::Dragons
    );
    assert_eq!(
        category(GameEvent::BaronKill {
            stolen: false,
            killer: killer.clone(),
            assisters: assisters.clone(),
        }),
        EventCategory::Barons
    );
    assert_eq!(
        category(GameEvent::HeraldKill {
            stolen: false,
            killer: killer.clone(),
            assisters: assisters.clone(),
        }),
        EventCategory::Heralds
    );
    assert_eq!(
        category(GameEvent::HordeKill {
            killer: killer.clone(),
            assisters: assisters.clone(),
        }),
        EventCategory::Heralds
    );
    assert_eq!(
        category(GameEvent::TurretKilled {
            turret: "Turret_T1_C_05_A".into(),
            killer: killer.clone(),
            assisters: assisters.clone(),
        }),
        EventCategory::Structures
    );
    assert_eq!(
        category(GameEvent::InhibKilled {
            inhib: "Barracks_T1_C1".into(),
            killer,
            assisters,
        }),
        EventCategory::Structures
    );
    assert_eq!(
        category(GameEvent::Ace {
            acer: "Foe".into(),
            acing_team: Team::Chaos
        }),
        EventCategory::Aces
    );
}

#[test]
fn flow_events_and_the_end_are_separate() {
    for kind in [
        GameEvent::GameStart,
        GameEvent::MinionsSpawning,
        GameEvent::FirstBlood {
            recipient: "Me".into(),
        },
        GameEvent::FirstBrick {
            killer: "Foe".into(),
        },
        GameEvent::InhibRespawningSoon {
            inhib: "Barracks_T1_C1".into(),
        },
        GameEvent::InhibRespawned {
            inhib: "Barracks_T1_C1".into(),
        },
        GameEvent::Unknown {
            name: "Mystery".into(),
        },
    ] {
        assert_eq!(category(kind), EventCategory::GameFlow);
    }
    assert_eq!(
        category(GameEvent::GameEnd {
            result: GameResult::Win
        }),
        EventCategory::GameEnd
    );
}

#[test]
fn the_recommended_set_keeps_me_objectives_and_the_end() {
    let set = CategorySet::recommended();
    for shown in [
        EventCategory::MyDeaths,
        EventCategory::MyKills,
        EventCategory::MyAssists,
        EventCategory::MyMultikills,
        EventCategory::Aces,
        EventCategory::Dragons,
        EventCategory::Barons,
        EventCategory::Heralds,
        EventCategory::Structures,
        EventCategory::GameEnd,
    ] {
        assert!(set.contains(shown), "{shown:?}");
    }
    for hidden in [
        EventCategory::TeammateKills,
        EventCategory::EnemyKills,
        EventCategory::OtherMultikills,
        EventCategory::GameFlow,
    ] {
        assert!(!set.contains(hidden), "{hidden:?}");
    }
    assert_eq!(CategorySet::default(), set);
}

#[test]
fn sets_toggle_and_list_in_order() {
    let set = CategorySet::NONE
        .with(EventCategory::Barons, true)
        .toggled(EventCategory::MyKills);
    assert_eq!(
        set.iter().collect::<Vec<_>>(),
        [EventCategory::MyKills, EventCategory::Barons]
    );
    assert!(
        !set.toggled(EventCategory::Barons)
            .contains(EventCategory::Barons)
    );
    assert_eq!(CategorySet::all().iter().count(), EventCategory::ALL.len());
}

#[test]
fn a_set_is_saved_as_a_list_of_snake_case_names() {
    let set = CategorySet::NONE
        .with(EventCategory::MyDeaths, true)
        .with(EventCategory::Heralds, true);
    let json = serde_json::to_string(&set).unwrap();
    assert_eq!(json, r#"["my_deaths","heralds"]"#);
    assert_eq!(serde_json::from_str::<CategorySet>(&json).unwrap(), set);
}

#[test]
fn unknown_names_in_a_saved_set_are_skipped() {
    let set: CategorySet = serde_json::from_str(r#"["my_kills","from_the_future",7]"#).unwrap();
    assert_eq!(set, CategorySet::NONE.with(EventCategory::MyKills, true));
    let wrong: CategorySet = serde_json::from_str(r#""oops""#).unwrap();
    assert_eq!(wrong, CategorySet::default());
}

#[test]
fn categorize_follows_the_event_order() {
    let entry = |id, kind| EventEntry {
        id,
        game_secs: 0.0,
        rec_secs: 0.0,
        kind,
        highlights: Vec::new(),
    };
    let events = [
        entry(1, kill("Me", "Foe", &[])),
        entry(2, GameEvent::GameStart),
    ];
    assert_eq!(
        categorize(&events, &info()),
        [EventCategory::MyKills, EventCategory::GameFlow]
    );
}

#[test]
fn the_groups_cover_every_category_once_in_order() {
    let grouped: Vec<EventCategory> = EventCategory::GROUPS
        .iter()
        .flat_map(|group| group.iter().copied())
        .collect();
    assert_eq!(grouped, EventCategory::ALL);
}
