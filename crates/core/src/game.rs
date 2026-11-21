use serde::{Deserialize, Serialize};

use crate::time::Hns;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Order,
    Chaos,
}

pub fn strip_tag(name: &str) -> &str {
    name.rsplit_once('#')
        .map_or(name, |(game_name, _)| game_name)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub champion: String,
    pub team: Team,
    pub is_bot: bool,
}

impl Player {
    pub fn has_name(&self, name: &str) -> bool {
        strip_tag(&self.name).eq_ignore_ascii_case(strip_tag(name))
    }

    fn has_champion(&self, name: &str) -> bool {
        self.champion.eq_ignore_ascii_case(name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameInfo {
    pub mode: String,
    pub map: Option<u32>,
    pub me: Player,
    pub players: Vec<Player>,
}

impl GameInfo {
    pub fn player_named(&self, name: &str) -> Option<&Player> {
        if self.me.has_name(name) {
            return Some(&self.me);
        }
        self.players
            .iter()
            .find(|player| player.has_name(name))
            .or_else(|| self.unique_champion(name))
    }

    pub fn is_me(&self, name: &str) -> bool {
        if self.me.has_name(name) {
            return true;
        }
        let named_other = self.players.iter().any(|player| player.has_name(name));
        !named_other && self.me.has_champion(name) && self.unique_champion(name).is_some()
    }

    fn unique_champion(&self, name: &str) -> Option<&Player> {
        let mut matching = self
            .players
            .iter()
            .filter(|player| player.has_champion(name));
        let first = matching.next()?;
        matching.next().is_none().then_some(first)
    }
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
