use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team};

const LOADING_SECS: f64 = 8.0;
const TAIL_SECS: f64 = 10.0;
const ME: &str = "Me#EUW";
const CHAMPIONS: [&str; 6] = ["Ahri", "Jinx", "Thresh", "Lee Sin", "Yasuo", "Lux"];
const MODES: [&str; 6] = [
    "CLASSIC",
    "CLASSIC",
    "ARAM",
    "PRACTICETOOL",
    "CLASSIC",
    "ARAM",
];
const RESULTS: [GameResult; 6] = [
    GameResult::Win,
    GameResult::Loss,
    GameResult::Win,
    GameResult::Unknown,
    GameResult::Loss,
    GameResult::Win,
];
const MINUTES: [f64; 6] = [6.0, 5.0, 4.0, 3.0, 7.0, 4.5];
const ALLIES: [(&str, &str); 4] = [
    ("Kaya#EUW", "Lee Sin"),
    ("Orin#EUW", "Jinx"),
    ("Tess#EUW", "Thresh"),
    ("Bram#EUW", "Ornn"),
];
const ENEMIES: [(&str, &str); 5] = [
    ("Vex#EUW", "Zed"),
    ("Lior#EUW", "Ashe"),
    ("Nova#EUW", "Leona"),
    ("Dax#EUW", "Viego"),
    ("Pike#EUW", "Darius"),
];

#[derive(Debug, Clone)]
pub(super) struct Spec {
    pub(super) index: usize,
    pub(super) champion: &'static str,
    pub(super) mode: &'static str,
    pub(super) result: GameResult,
    pub(super) media_secs: f64,
    pub(super) hours_ago: i64,
}

pub(super) fn specs(count: usize) -> Vec<Spec> {
    let spread = 116 / i64::try_from(count.max(1)).unwrap_or(1);
    (0..count)
        .map(|index| {
            let slot = index % CHAMPIONS.len();
            let step = i64::try_from(index).unwrap_or(0);
            Spec {
                index,
                champion: CHAMPIONS[slot],
                mode: MODES[slot],
                result: RESULTS[slot],
                media_secs: MINUTES[slot] * 60.0,
                hours_ago: 2 + step * spread + step * 3 % 5,
            }
        })
        .collect()
}

fn player(name: &str, champion: &str, team: Team) -> Player {
    Player {
        name: name.to_owned(),
        champion: champion.to_owned(),
        team,
        is_bot: false,
    }
}

pub(super) fn game_info(spec: &Spec) -> GameInfo {
    let me = player(ME, spec.champion, Team::Order);
    let mut players = vec![me.clone()];
    if spec.mode != "PRACTICETOOL" {
        for (name, champion) in ALLIES {
            let champion = if champion == spec.champion {
                "Ahri"
            } else {
                champion
            };
            players.push(player(name, champion, Team::Order));
        }
        for (name, champion) in ENEMIES {
            players.push(player(name, champion, Team::Chaos));
        }
    }
    GameInfo {
        mode: spec.mode.to_owned(),
        map: Some(if spec.mode == "ARAM" { 12 } else { 11 }),
        me,
        players,
    }
}

pub(super) fn loading_secs() -> f64 {
    LOADING_SECS
}

fn names(list: &[(&str, &str)]) -> Vec<String> {
    list.iter().map(|(name, _)| (*name).to_owned()).collect()
}

fn kill(killer: &str, victim: &str, assisters: &[&str]) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.to_owned(),
        victim: victim.to_owned(),
        assisters: assisters.iter().map(|a| (*a).to_owned()).collect(),
    }
}

pub(super) fn timeline(spec: &Spec) -> Vec<(f64, GameEvent)> {
    let length = (spec.media_secs - LOADING_SECS - TAIL_SECS).max(30.0);
    let shift = f64::from(i32::try_from(spec.index * 7 % 5).unwrap_or(0) - 2) * 0.01;
    let at =
        |fraction: f64| ((fraction + shift).clamp(0.02, 0.97) * length * 100.0).round() / 100.0;
    let mut events = vec![(0.03, GameEvent::GameStart)];
    if spec.mode == "PRACTICETOOL" {
        for fraction in [0.2, 0.4, 0.6, 0.8] {
            events.push((at(fraction), kill(ME, "Target Dummy", &[])));
        }
        return sorted(events);
    }
    let allies = names(&ALLIES);
    let enemies = names(&ENEMIES);
    let win = spec.result == GameResult::Win;
    let classic = spec.mode == "CLASSIC";
    events.push((at(0.03).max(1.0), GameEvent::MinionsSpawning));
    events.push((
        at(0.10),
        GameEvent::FirstBlood {
            recipient: ME.to_owned(),
        },
    ));
    events.push((at(0.10), kill(ME, &enemies[0], &[&allies[0]])));
    events.push((at(0.18), kill(&allies[1], &enemies[2], &[ME])));
    events.push((at(0.25), kill(&enemies[1], ME, &[&enemies[3]])));
    if classic {
        events.push((
            at(0.32),
            GameEvent::DragonKill {
                dragon_type: "Earth".to_owned(),
                stolen: false,
                killer: allies[2].clone(),
                assisters: vec![ME.to_owned()],
            },
        ));
    }
    events.push((
        at(0.45),
        GameEvent::TurretKilled {
            turret: "Turret_T2_L_03_A".to_owned(),
            killer: ME.to_owned(),
            assisters: vec![allies[0].clone()],
        },
    ));
    events.push((at(0.54), kill(ME, &enemies[2], &[])));
    events.push((at(0.56), kill(ME, &enemies[4], &[&allies[3]])));
    events.push((
        at(0.56) + 0.5,
        GameEvent::Multikill {
            killer: ME.to_owned(),
            streak: 2,
        },
    ));
    events.push((at(0.66), kill(&enemies[0], &allies[1], &[])));
    if classic {
        events.push((
            at(0.76),
            GameEvent::BaronKill {
                stolen: false,
                killer: if win { &allies[0] } else { &enemies[1] }.clone(),
                assisters: Vec::new(),
            },
        ));
    }
    events.push((
        at(0.86),
        GameEvent::Ace {
            acer: if win { ME } else { &enemies[0] }.to_owned(),
            acing_team: if win { Team::Order } else { Team::Chaos },
        },
    ));
    events.push((
        at(0.92),
        GameEvent::InhibKilled {
            inhib: if win {
                "Barracks_T2_C1"
            } else {
                "Barracks_T1_C1"
            }
            .to_owned(),
            killer: if win { ME } else { &enemies[3] }.to_owned(),
            assisters: Vec::new(),
        },
    ));
    events.push((
        length,
        GameEvent::GameEnd {
            result: spec.result,
        },
    ));
    sorted(events)
}

fn sorted(mut events: Vec<(f64, GameEvent)>) -> Vec<(f64, GameEvent)> {
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    events
}
