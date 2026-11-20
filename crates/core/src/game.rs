use serde::{Deserialize, Serialize};

use crate::time::Hns;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Order,
    Chaos,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub champion: String,
    pub team: Team,
    pub is_bot: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameInfo {
    pub mode: String,
    pub map: Option<u32>,
    pub me: Player,
    pub players: Vec<Player>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameResult {
    Win,
    Loss,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    GameStart,
    MinionsSpawning,
    FirstBlood {
        recipient: String,
    },
    ChampionKill {
        killer: String,
        victim: String,
        assisters: Vec<String>,
    },
    Multikill {
        killer: String,
        streak: u8,
    },
    Ace {
        acer: String,
        acing_team: Team,
    },
    FirstBrick {
        killer: String,
    },
    TurretKilled {
        turret: String,
        killer: String,
        assisters: Vec<String>,
    },
    InhibKilled {
        inhib: String,
        killer: String,
        assisters: Vec<String>,
    },
    InhibRespawningSoon {
        inhib: String,
    },
    InhibRespawned {
        inhib: String,
    },
    DragonKill {
        dragon_type: String,
        stolen: bool,
        killer: String,
        assisters: Vec<String>,
    },
    HeraldKill {
        stolen: bool,
        killer: String,
        assisters: Vec<String>,
    },
    BaronKill {
        stolen: bool,
        killer: String,
        assisters: Vec<String>,
    },
    HordeKill {
        killer: String,
        assisters: Vec<String>,
    },
    GameEnd {
        result: GameResult,
    },
    Unknown {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimedEvent {
    pub id: u32,
    pub game_secs: f64,
    pub at: Hns,
    pub kind: GameEvent,
}

#[cfg(test)]
mod tests;
