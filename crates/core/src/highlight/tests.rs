use super::*;
use crate::game::Player;

fn player(name: &str, champion: &str, team: Team, is_bot: bool) -> Player {
    Player {
        name: name.into(),
        champion: champion.into(),
        team,
        is_bot,
    }
}

fn info() -> GameInfo {
    let me = player("Faker", "Ahri", Team::Order, false);
    GameInfo {
        mode: "CLASSIC".into(),
        map: Some(11),
        me: me.clone(),
        players: vec![
            me,
            player("Keria", "Lux", Team::Order, false),
            player("Chovy", "Zed", Team::Chaos, false),
            player("Garen", "Garen", Team::Chaos, true),
        ],
    }
}

fn kill(killer: &str, victim: &str, assisters: &[&str]) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.into(),
        victim: victim.into(),
        assisters: assisters.iter().map(|name| (*name).to_string()).collect(),
    }
}

fn all_toggles(on: bool) -> ClipSettings {
    ClipSettings {
        kills: on,
        deaths: on,
        assists: on,
        multikills: on,
        objectives: on,
        structures: on,
        aces: on,
        ..ClipSettings::default()
    }
}

#[test]
fn my_kill() {
    assert_eq!(
        classify(&kill("Faker", "Chovy", &["Keria"]), &info()),
        vec![Highlight::MyKill]
    );
}

#[test]
fn my_death() {
    assert_eq!(
        classify(&kill("Chovy", "Faker", &[]), &info()),
        vec![Highlight::MyDeath]
    );
}

#[test]
fn my_assist() {
    assert_eq!(
        classify(&kill("Keria", "Chovy", &["Faker"]), &info()),
        vec![Highlight::MyAssist]
    );
}

#[test]
fn kill_and_assist_returns_both() {
    assert_eq!(
        classify(&kill("Faker", "Chovy", &["Faker"]), &info()),
        vec![Highlight::MyKill, Highlight::MyAssist]
    );
}

#[test]
fn environment_killer_still_gives_my_death() {
    assert_eq!(
        classify(&kill("Turret_T2_L_03_A", "Faker", &[]), &info()),
        vec![Highlight::MyDeath]
    );
}

#[test]
fn environment_killer_of_other_player_is_ignored() {
    assert!(classify(&kill("Minion_T200L0S1", "Keria", &[]), &info()).is_empty());
}

#[test]
fn kills_between_others_are_ally_or_enemy() {
    assert_eq!(
        classify(&kill("Keria", "Chovy", &[]), &info()),
        vec![Highlight::AllyKill]
    );
    assert_eq!(
        classify(&kill("Chovy", "Keria", &[]), &info()),
        vec![Highlight::EnemyKill]
    );
}

#[test]
fn name_matching_ignores_case_and_tag() {
    assert_eq!(
        classify(&kill("faker#KR1", "Chovy", &[]), &info()),
        vec![Highlight::MyKill]
    );
    assert_eq!(
        classify(&kill("Chovy", "FAKER", &[]), &info()),
        vec![Highlight::MyDeath]
    );
}

#[test]
fn champion_named_events_match_by_champion_name() {
    let mut info = info();
    info.me = player("Bot Ahri", "Ahri", Team::Order, true);
    info.players[0] = info.me.clone();
    assert_eq!(
        classify(&kill("Ahri", "Chovy", &[]), &info),
        vec![Highlight::MyKill]
    );
    let baron = GameEvent::BaronKill {
        stolen: false,
        killer: "Garen".into(),
        assisters: vec![],
    };
    assert_eq!(
        classify(&baron, &info),
        vec![Highlight::EnemyObjective(Objective::Baron)]
    );
}

#[test]
fn first_blood_only_for_me() {
    let mine = GameEvent::FirstBlood {
        recipient: "Faker".into(),
    };
    let theirs = GameEvent::FirstBlood {
        recipient: "Chovy".into(),
    };
    assert_eq!(classify(&mine, &info()), vec![Highlight::MyFirstBlood]);
    assert!(classify(&theirs, &info()).is_empty());
}

#[test]
fn multikill_only_for_me() {
    let multikill = |killer: &str| GameEvent::Multikill {
        killer: killer.into(),
        streak: 3,
    };
    assert_eq!(
        classify(&multikill("Faker"), &info()),
        vec![Highlight::MyMultikill { streak: 3 }]
    );
    assert!(classify(&multikill("Chovy"), &info()).is_empty());
}

#[test]
fn objectives_split_by_killer_team() {
    let dragon = |killer: &str| GameEvent::DragonKill {
        dragon_type: "Earth".into(),
        stolen: false,
        killer: killer.into(),
        assisters: vec![],
    };
    let earth = || Objective::Dragon("Earth".into());
    assert_eq!(
        classify(&dragon("Keria"), &info()),
        vec![Highlight::TeamObjective(earth())]
    );
    assert_eq!(
        classify(&dragon("Chovy"), &info()),
        vec![Highlight::EnemyObjective(earth())]
    );
    assert!(classify(&dragon("SRU_Baron00"), &info()).is_empty());

    let herald = GameEvent::HeraldKill {
        stolen: false,
        killer: "Faker".into(),
        assisters: vec![],
    };
    assert_eq!(
        classify(&herald, &info()),
        vec![Highlight::TeamObjective(Objective::Herald)]
    );
    let grub = GameEvent::HordeKill {
        killer: "Chovy".into(),
        assisters: vec![],
    };
    assert_eq!(
        classify(&grub, &info()),
        vec![Highlight::EnemyObjective(Objective::VoidGrub)]
    );
}

#[test]
fn structures_use_the_id_team() {
    let turret = |id: &str| GameEvent::TurretKilled {
        turret: id.into(),
        killer: "Chovy".into(),
        assisters: vec![],
    };
    assert_eq!(
        classify(&turret("Turret_T2_L_03_A"), &info()),
        vec![Highlight::TeamStructure(Structure::Turret)]
    );
    assert_eq!(
        classify(&turret("Turret_T1_C_01_A"), &info()),
        vec![Highlight::EnemyStructure(Structure::Turret)]
    );
    let inhib = GameEvent::InhibKilled {
        inhib: "Barracks_T1_R1".into(),
        killer: "Chovy".into(),
        assisters: vec![],
    };
    assert_eq!(
        classify(&inhib, &info()),
        vec![Highlight::EnemyStructure(Structure::Inhibitor)]
    );
    assert!(classify(&turret("Turret_Unknown"), &info()).is_empty());
}

#[test]
fn structure_team_reads_research_ids() {
    assert_eq!(structure_team("Turret_T1_L_03_A"), Some(Team::Order));
    assert_eq!(structure_team("Barracks_T2_L1"), Some(Team::Chaos));
    assert_eq!(structure_team("Turret_T200_X"), None);
    assert_eq!(structure_team(""), None);
}

#[test]
fn ace_splits_by_team() {
    let ace = |team| GameEvent::Ace {
        acer: "Chovy".into(),
        acing_team: team,
    };
    assert_eq!(classify(&ace(Team::Order), &info()), vec![Highlight::MyAce]);
    assert_eq!(
        classify(&ace(Team::Chaos), &info()),
        vec![Highlight::EnemyAce]
    );
}

#[test]
fn game_end_maps_result() {
    let end = |result| GameEvent::GameEnd { result };
    assert_eq!(
        classify(&end(GameResult::Win), &info()),
        vec![Highlight::GameWon]
    );
    assert_eq!(
        classify(&end(GameResult::Loss), &info()),
        vec![Highlight::GameLost]
    );
    assert!(classify(&end(GameResult::Unknown), &info()).is_empty());
}

#[test]
fn other_events_are_ignored() {
    for event in [
        GameEvent::GameStart,
        GameEvent::MinionsSpawning,
        GameEvent::FirstBrick {
            killer: "Faker".into(),
        },
        GameEvent::InhibRespawned {
            inhib: "Barracks_T1_R1".into(),
        },
        GameEvent::Unknown {
            name: "AtakhanKill".into(),
        },
    ] {
        assert!(classify(&event, &info()).is_empty());
    }
}

#[test]
fn labels() {
    assert_eq!(Highlight::MyKill.label(), "MyKill");
    assert_eq!(Highlight::MyMultikill { streak: 2 }.label(), "Multikill2");
    assert_eq!(Highlight::MyMultikill { streak: 5 }.label(), "Multikill5");
    assert_eq!(
        Highlight::TeamObjective(Objective::Dragon("Fire".into())).label(),
        "Dragon"
    );
    assert_eq!(
        Highlight::EnemyObjective(Objective::Baron).label(),
        "EnemyBaron"
    );
    assert_eq!(
        Highlight::TeamStructure(Structure::Inhibitor).label(),
        "Inhibitor"
    );
    assert_eq!(Highlight::MyAce.label(), "Ace");
    assert_eq!(Highlight::GameWon.label(), "Victory");
    assert_eq!(Highlight::GameLost.label(), "Defeat");
}

#[test]
fn clip_worthy_follows_toggles() {
    let off = all_toggles(false);
    let cases = [
        (Highlight::MyKill, ClipSettings { kills: true, ..off }),
        (Highlight::MyFirstBlood, ClipSettings { kills: true, ..off }),
        (
            Highlight::MyDeath,
            ClipSettings {
                deaths: true,
                ..off
            },
        ),
        (
            Highlight::MyAssist,
            ClipSettings {
                assists: true,
                ..off
            },
        ),
        (
            Highlight::MyMultikill { streak: 2 },
            ClipSettings {
                multikills: true,
                ..off
            },
        ),
        (
            Highlight::TeamObjective(Objective::Baron),
            ClipSettings {
                objectives: true,
                ..off
            },
        ),
        (
            Highlight::EnemyObjective(Objective::Herald),
            ClipSettings {
                objectives: true,
                ..off
            },
        ),
        (
            Highlight::TeamStructure(Structure::Turret),
            ClipSettings {
                structures: true,
                ..off
            },
        ),
        (
            Highlight::EnemyStructure(Structure::Inhibitor),
            ClipSettings {
                structures: true,
                ..off
            },
        ),
        (Highlight::MyAce, ClipSettings { aces: true, ..off }),
    ];
    for (highlight, only_on) in cases {
        assert!(highlight.is_clip_worthy(&only_on), "{highlight:?}");
        assert!(!highlight.is_clip_worthy(&off), "{highlight:?}");
    }
    for highlight in [
        Highlight::AllyKill,
        Highlight::EnemyKill,
        Highlight::EnemyAce,
        Highlight::GameWon,
        Highlight::GameLost,
    ] {
        assert!(
            !highlight.is_clip_worthy(&all_toggles(true)),
            "{highlight:?}"
        );
    }
}
