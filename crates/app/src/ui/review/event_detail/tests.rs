use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team};
use greyscreen_core::library::EventEntry;

use super::{Detail, Span, describe, streak_kills, structure_name};

fn player(name: &str, champion: &str, team: Team) -> Player {
    Player {
        name: name.into(),
        champion: champion.into(),
        team,
        is_bot: false,
    }
}

fn info() -> GameInfo {
    let me = player("Summoner", "Ahri", Team::Order);
    GameInfo {
        mode: "CLASSIC".into(),
        map: Some(11),
        players: vec![
            me.clone(),
            player("Ally#EUW", "Jinx", Team::Order),
            player("Foe", "Zed", Team::Chaos),
            player("Rival", "Lux", Team::Chaos),
        ],
        me,
    }
}

fn entry(id: u32, rec_secs: f64, kind: GameEvent) -> EventEntry {
    EventEntry {
        id,
        game_secs: rec_secs,
        rec_secs,
        kind,
        highlights: Vec::new(),
    }
}

fn kill(id: u32, rec_secs: f64, killer: &str, victim: &str) -> EventEntry {
    entry(
        id,
        rec_secs,
        GameEvent::ChampionKill {
            killer: killer.into(),
            victim: victim.into(),
            assisters: Vec::new(),
        },
    )
}

fn plain(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|s| match s {
            Span::Strong(t) | Span::Muted(t) => t.as_str(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn rows(detail: &Detail) -> Vec<String> {
    detail
        .lines
        .iter()
        .map(|l| format!("{}|{}", l.lead, plain(&l.spans)))
        .collect()
}

#[test]
fn a_champion_kill_names_champions_first_and_players_after() {
    let event = entry(
        1,
        65.0,
        GameEvent::ChampionKill {
            killer: "Foe".into(),
            victim: "Summoner".into(),
            assisters: vec!["Rival".into()],
        },
    );
    let detail = describe(&event, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "Zed Foe killed You");
    assert_eq!(detail.time, "01:05");
    assert_eq!(rows(&detail), ["Assist|Lux Rival"]);
}

#[test]
fn the_local_player_reads_you_even_with_a_tag() {
    let event = kill(1, 5.0, "Summoner#EUW", "Foe");
    let detail = describe(&event, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "You killed Zed Foe");
}

#[test]
fn a_multikill_lists_the_last_streak_kills_of_that_killer_in_order() {
    let events = vec![
        kill(1, 10.0, "Ally", "Foe"),
        kill(2, 20.0, "Ally", "Rival"),
        kill(3, 25.0, "Foe", "Summoner"),
        kill(4, 30.0, "Ally", "Foe"),
        kill(5, 40.0, "Ally", "Rival"),
        kill(6, 50.0, "Ally", "Foe"),
        kill(7, 90.0, "Ally", "Rival"),
    ];
    let multikill = entry(
        8,
        50.0,
        GameEvent::Multikill {
            killer: "Ally".into(),
            streak: 3,
        },
    );
    let found: Vec<u32> = streak_kills(&events, &multikill, "Ally", 3)
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(found, [4, 5, 6]);

    let detail = describe(&multikill, &events, &info()).unwrap();
    assert_eq!(plain(&detail.title), "Triple kill by Jinx Ally");
    assert_eq!(
        rows(&detail),
        ["00:30|Zed Foe", "00:40|Lux Rival", "00:50|Zed Foe"]
    );
}

#[test]
fn a_multikill_with_fewer_recorded_kills_lists_what_exists() {
    let events = vec![kill(1, 10.0, "Summoner", "Foe")];
    let multikill = entry(
        2,
        10.0,
        GameEvent::Multikill {
            killer: "Summoner".into(),
            streak: 5,
        },
    );
    let detail = describe(&multikill, &events, &info()).unwrap();
    assert_eq!(plain(&detail.title), "Pentakill by You");
    assert_eq!(rows(&detail), ["00:10|Zed Foe"]);
}

#[test]
fn live_client_structure_names_decode_to_side_lane_and_tier() {
    let name = |id: &str| structure_name(id, Some(11));
    assert_eq!(
        name("Turret_T1_R_03_A").as_deref(),
        Some("Blue bot outer turret")
    );
    assert_eq!(
        name("Turret_T2_L_02_A").as_deref(),
        Some("Red top inner turret")
    );
    assert_eq!(
        name("Turret_T2_L_01_A").as_deref(),
        Some("Red top inhibitor turret")
    );
    assert_eq!(
        name("Turret_T1_C_05_A").as_deref(),
        Some("Blue mid outer turret")
    );
    assert_eq!(
        name("Turret_T1_C_04_A").as_deref(),
        Some("Blue mid inner turret")
    );
    assert_eq!(
        name("Turret_T2_C_03_A").as_deref(),
        Some("Red mid inhibitor turret")
    );
    assert_eq!(
        name("Turret_T2_C_01_A").as_deref(),
        Some("Red nexus turret")
    );
    assert_eq!(name("Barracks_T2_L1").as_deref(), Some("Red top inhibitor"));
    assert_eq!(
        name("Barracks_T1_C1").as_deref(),
        Some("Blue mid inhibitor")
    );
    assert_eq!(
        name("Barracks_T1_R1").as_deref(),
        Some("Blue bot inhibitor")
    );
}

#[test]
fn unknown_structure_names_stay_raw() {
    assert_eq!(structure_name("Turret_T9_Q_03_A", Some(11)), None);
    assert_eq!(structure_name("Turret_T1_Q_03_A", Some(11)), None);
    assert_eq!(structure_name("Turret_T1_L_09_A", Some(11)), None);
    assert_eq!(structure_name("Nexus_T1", Some(11)), None);
    let event = entry(
        1,
        5.0,
        GameEvent::TurretKilled {
            turret: "Turret_Odd".into(),
            killer: "SRU_Baron".into(),
            assisters: Vec::new(),
        },
    );
    let detail = describe(&event, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "Turret destroyed");
    assert_eq!(rows(&detail), ["Killed by|SRU_Baron"]);
}

fn aram_info() -> GameInfo {
    GameInfo {
        mode: "KIWI".into(),
        map: Some(12),
        ..info()
    }
}

#[test]
fn howling_abyss_structure_ids_decode_to_side_and_tier() {
    let name = |id: &str| structure_name(id, Some(12));
    assert_eq!(
        name("Turret_TChaos_L1_P3_671196429_0").as_deref(),
        Some("Red outer turret")
    );
    assert_eq!(
        name("Turret_TChaos_L1_P2_620863572_0").as_deref(),
        Some("Red inner turret")
    );
    assert_eq!(
        name("Turret_TOrder_L1_P4_587308334_0").as_deref(),
        Some("Blue nexus turret")
    );
    assert_eq!(
        name("Turret_TChaos_L1_P5_637641191_0").as_deref(),
        Some("Red nexus turret")
    );
    assert_eq!(
        name("Inhib_TChaos_L1_P1_2432143522_0").as_deref(),
        Some("Red inhibitor")
    );
}

#[test]
fn modern_structure_ids_on_other_maps_name_only_the_side() {
    assert_eq!(
        structure_name("Turret_TOrder_L1_P3_671196429_0", Some(11)).as_deref(),
        Some("Blue turret")
    );
}

#[test]
fn a_howling_abyss_turret_card_never_shows_the_raw_id() {
    let event = entry(
        1,
        640.0,
        GameEvent::TurretKilled {
            turret: "Turret_TChaos_L1_P3_671196429_0".into(),
            killer: "Foe".into(),
            assisters: Vec::new(),
        },
    );
    let detail = describe(&event, &[], &aram_info()).unwrap();
    assert_eq!(plain(&detail.title), "Red outer turret destroyed");
}

#[test]
fn unrecognised_structures_still_get_a_side_and_a_kind() {
    let event = entry(
        1,
        640.0,
        GameEvent::InhibKilled {
            inhib: "Inhib_TOrder_X".into(),
            killer: "Foe".into(),
            assisters: Vec::new(),
        },
    );
    let detail = describe(&event, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "Blue inhibitor destroyed");
}

#[test]
fn a_turret_card_names_the_structure_killer_and_assisters() {
    let event = entry(
        1,
        200.0,
        GameEvent::TurretKilled {
            turret: "Turret_T2_C_05_A".into(),
            killer: "Ally".into(),
            assisters: vec!["Summoner".into()],
        },
    );
    let detail = describe(&event, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "Red mid outer turret destroyed");
    assert_eq!(rows(&detail), ["Killed by|Jinx Ally", "Assist|You"]);
}

#[test]
fn minions_and_turrets_can_be_the_killer() {
    let by = |killer: &str| {
        let event = entry(
            1,
            9.0,
            GameEvent::InhibKilled {
                inhib: "Barracks_T1_L1".into(),
                killer: killer.into(),
                assisters: Vec::new(),
            },
        );
        rows(&describe(&event, &[], &info()).unwrap())
    };
    assert_eq!(by("Minion_T200L0S1"), ["Killed by|Minions"]);
    assert_eq!(by("Turret_T1_R_03_A"), ["Killed by|Blue bot outer turret"]);
}

#[test]
fn objectives_show_the_type_killer_and_a_stolen_flag() {
    let dragon = entry(
        1,
        300.0,
        GameEvent::DragonKill {
            dragon_type: "Fire".into(),
            stolen: true,
            killer: "Foe".into(),
            assisters: vec!["Rival".into()],
        },
    );
    let detail = describe(&dragon, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "Fire dragon");
    assert_eq!(
        rows(&detail),
        ["|Stolen", "Slain by|Zed Foe", "Assist|Lux Rival"]
    );

    let grub = entry(
        2,
        310.0,
        GameEvent::HordeKill {
            killer: "Summoner".into(),
            assisters: Vec::new(),
        },
    );
    let detail = describe(&grub, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "Void Grub");
    assert_eq!(rows(&detail), ["Slain by|You"]);
}

#[test]
fn first_blood_ace_and_game_end_have_cards() {
    let blood = entry(
        1,
        90.0,
        GameEvent::FirstBlood {
            recipient: "Summoner".into(),
        },
    );
    assert_eq!(
        rows(&describe(&blood, &[], &info()).unwrap()),
        ["Taken by|You"]
    );

    let ace = entry(
        2,
        900.0,
        GameEvent::Ace {
            acer: "Foe".into(),
            acing_team: Team::Chaos,
        },
    );
    let detail = describe(&ace, &[], &info()).unwrap();
    assert_eq!(plain(&detail.title), "Ace Red team");
    assert_eq!(rows(&detail), ["Acer|Zed Foe"]);

    let end = entry(
        3,
        1800.0,
        GameEvent::GameEnd {
            result: GameResult::Win,
        },
    );
    assert_eq!(
        plain(&describe(&end, &[], &info()).unwrap().title),
        "Victory"
    );
}

#[test]
fn unknown_events_have_no_card() {
    let event = entry(1, 1.0, GameEvent::Unknown { name: "X".into() });
    assert_eq!(describe(&event, &[], &info()), None);
}
