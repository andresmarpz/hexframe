use greyscreen_core::game::{GameEvent, GameInfo, GameResult};
use greyscreen_core::highlight::classify;
use greyscreen_core::library::EventEntry;

const GAME_OFFSET_SECS: f64 = 8.0;
const FIRST_FIGHT_SECS: f64 = 240.0;
const LAST_FIGHT_MARGIN_SECS: f64 = 90.0;
const DOUBLE_KILL_GAP_SECS: f64 = 12.0;
const MINUTE: f64 = 60.0;
const SPREAD: usize = 7;
const ALLY_KILLS: usize = 6;

pub(super) struct Shape {
    pub minutes: f64,
    pub kills: usize,
    pub deaths: usize,
    pub assists: usize,
    pub rift: bool,
    pub result: GameResult,
}

fn me() -> String {
    "Summoner".into()
}

fn kill(killer: &str, victim: &str, assisters: &[&str]) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.into(),
        victim: victim.into(),
        assisters: assisters.iter().map(|name| (*name).to_owned()).collect(),
    }
}

fn dragon(kind: &str, killer: &str) -> GameEvent {
    GameEvent::DragonKill {
        dragon_type: kind.into(),
        stolen: false,
        killer: killer.into(),
        assisters: Vec::new(),
    }
}

fn turret(id: &str, killer: &str) -> GameEvent {
    GameEvent::TurretKilled {
        turret: id.into(),
        killer: killer.into(),
        assisters: Vec::new(),
    }
}

fn is_my_kill(event: &GameEvent) -> bool {
    matches!(event, GameEvent::ChampionKill { killer, .. } if killer == "Summoner")
}

fn fights(shape: &Shape) -> Vec<(f64, GameEvent)> {
    let mut pool: Vec<GameEvent> = Vec::new();
    pool.extend((0..shape.kills).map(|_| kill("Summoner", "Zed", &[])));
    pool.extend((0..shape.deaths).map(|_| kill("Lee Sin", "Summoner", &[])));
    pool.extend((0..shape.assists).map(|_| kill("Jinx", "Lux", &["Summoner"])));
    pool.extend((0..ALLY_KILLS).map(|_| kill("Jinx", "Zed", &[])));
    let total = pool.len();
    let mut order: Vec<(usize, GameEvent)> = pool.into_iter().enumerate().collect();
    order.sort_by_key(|(index, _)| (index * SPREAD) % total.max(1));
    let span = (shape.minutes * MINUTE - FIRST_FIGHT_SECS - LAST_FIGHT_MARGIN_SECS).max(MINUTE);
    order
        .into_iter()
        .enumerate()
        .map(|(slot, (_, event))| {
            let at = FIRST_FIGHT_SECS + span * (slot as f64 + 0.5) / total.max(1) as f64;
            (at, event)
        })
        .collect()
}

fn double_kill(rows: &mut [(f64, GameEvent)]) -> Option<(f64, GameEvent)> {
    let mut mine = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, event))| is_my_kill(event))
        .map(|(index, _)| index);
    let (first, second) = (mine.next()?, mine.next()?);
    let at = rows[first].0 + DOUBLE_KILL_GAP_SECS;
    rows[second].0 = at;
    let streak = GameEvent::Multikill {
        killer: me(),
        streak: 2,
    };
    Some((at, streak))
}

fn my_team(event: fn(String) -> GameEvent) -> GameEvent {
    event(me())
}

fn herald(killer: String) -> GameEvent {
    GameEvent::HeraldKill {
        stolen: false,
        killer,
        assisters: Vec::new(),
    }
}

fn baron(killer: String) -> GameEvent {
    GameEvent::BaronKill {
        stolen: false,
        killer,
        assisters: Vec::new(),
    }
}

fn objectives(shape: &Shape) -> Vec<(f64, GameEvent)> {
    let end = shape.minutes * MINUTE;
    let mut rows = vec![
        (end * 0.30, turret("Turret_T2_C_05_A", "Summoner")),
        (end * 0.45, turret("Turret_T1_R_03_A", "Lee Sin")),
    ];
    if !shape.rift {
        return rows;
    }
    rows.extend([
        (end * 0.18, dragon("Fire", "Jinx")),
        (end * 0.27, dragon("Earth", "Lee Sin")),
        (end * 0.36, my_team(herald)),
        (end * 0.62, turret("Turret_T2_L_01_A", "Jinx")),
    ]);
    if shape.minutes > 28.0 {
        rows.extend([
            (end * 0.80, my_team(baron)),
            (end * 0.88, dragon("Elder", "Summoner")),
            (
                end * 0.94,
                GameEvent::InhibKilled {
                    inhib: "Barracks_T2_C1".into(),
                    killer: me(),
                    assisters: Vec::new(),
                },
            ),
        ]);
    }
    rows
}

pub(super) fn build(shape: &Shape, info: &GameInfo) -> Vec<EventEntry> {
    let mut rows = fights(shape);
    let double = double_kill(&mut rows);
    rows.extend(double);
    rows.extend(objectives(shape));
    rows.push((
        shape.minutes * MINUTE - 10.0,
        GameEvent::GameEnd {
            result: shape.result,
        },
    ));
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter()
        .zip(1u32..)
        .map(|((game_secs, kind), id)| EventEntry {
            id,
            game_secs,
            rec_secs: game_secs + GAME_OFFSET_SECS,
            highlights: classify(&kind, info)
                .iter()
                .map(|highlight| highlight.label().to_owned())
                .collect(),
            kind,
        })
        .collect()
}
