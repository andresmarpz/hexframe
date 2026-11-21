use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::game::{GameEvent, GameInfo, Team};
use crate::library::EventEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    MyDeaths,
    MyKills,
    MyAssists,
    MyMultikills,
    TeammateKills,
    EnemyKills,
    OtherMultikills,
    Aces,
    Dragons,
    Barons,
    Heralds,
    Structures,
    GameFlow,
    GameEnd,
}

impl EventCategory {
    pub const GROUPS: [&'static [EventCategory]; 4] = [
        &[
            EventCategory::MyDeaths,
            EventCategory::MyKills,
            EventCategory::MyAssists,
            EventCategory::MyMultikills,
        ],
        &[
            EventCategory::TeammateKills,
            EventCategory::EnemyKills,
            EventCategory::OtherMultikills,
        ],
        &[
            EventCategory::Aces,
            EventCategory::Dragons,
            EventCategory::Barons,
            EventCategory::Heralds,
            EventCategory::Structures,
        ],
        &[EventCategory::GameFlow, EventCategory::GameEnd],
    ];

    pub const ALL: [EventCategory; 14] = [
        EventCategory::MyDeaths,
        EventCategory::MyKills,
        EventCategory::MyAssists,
        EventCategory::MyMultikills,
        EventCategory::TeammateKills,
        EventCategory::EnemyKills,
        EventCategory::OtherMultikills,
        EventCategory::Aces,
        EventCategory::Dragons,
        EventCategory::Barons,
        EventCategory::Heralds,
        EventCategory::Structures,
        EventCategory::GameFlow,
        EventCategory::GameEnd,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            EventCategory::MyDeaths => "My deaths",
            EventCategory::MyKills => "My kills",
            EventCategory::MyAssists => "My assists",
            EventCategory::TeammateKills => "Teammates' kills",
            EventCategory::EnemyKills => "Enemies' kills",
            EventCategory::MyMultikills => "My multikills",
            EventCategory::OtherMultikills => "Others' multikills",
            EventCategory::Aces => "Aces",
            EventCategory::Dragons => "Dragons",
            EventCategory::Barons => "Barons",
            EventCategory::Heralds => "Heralds and void grubs",
            EventCategory::Structures => "Towers and inhibitors",
            EventCategory::GameFlow => "Game flow",
            EventCategory::GameEnd => "Game end",
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            EventCategory::MyDeaths => "my-deaths",
            EventCategory::MyKills => "my-kills",
            EventCategory::MyAssists => "my-assists",
            EventCategory::TeammateKills => "teammate-kills",
            EventCategory::EnemyKills => "enemy-kills",
            EventCategory::MyMultikills => "my-multikills",
            EventCategory::OtherMultikills => "other-multikills",
            EventCategory::Aces => "aces",
            EventCategory::Dragons => "dragons",
            EventCategory::Barons => "barons",
            EventCategory::Heralds => "heralds",
            EventCategory::Structures => "structures",
            EventCategory::GameFlow => "game-flow",
            EventCategory::GameEnd => "game-end",
        }
    }

    pub const fn search_terms(self) -> &'static str {
        match self {
            EventCategory::MyDeaths => "my deaths died slain",
            EventCategory::MyKills => "my kills killed",
            EventCategory::MyAssists => "my assists",
            EventCategory::TeammateKills => "teammates allies kills",
            EventCategory::EnemyKills => "enemies enemy kills",
            EventCategory::MyMultikills => "my multikills double triple quadra penta",
            EventCategory::OtherMultikills => "others multikills double triple quadra penta",
            EventCategory::Aces => "aces ace",
            EventCategory::Dragons => "dragons drakes elder",
            EventCategory::Barons => "barons nashor",
            EventCategory::Heralds => "heralds rift herald void grubs horde",
            EventCategory::Structures => "towers turrets inhibitors structures nexus",
            EventCategory::GameFlow => "game flow start minions spawning first blood brick",
            EventCategory::GameEnd => "game end victory defeat result",
        }
    }

    const fn bit(self) -> u16 {
        1 << self as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CategorySet(u16);

impl CategorySet {
    pub const NONE: CategorySet = CategorySet(0);

    pub fn all() -> CategorySet {
        EventCategory::ALL
            .into_iter()
            .fold(CategorySet::NONE, |set, category| set.with(category, true))
    }

    pub fn recommended() -> CategorySet {
        let quiet = [
            EventCategory::TeammateKills,
            EventCategory::EnemyKills,
            EventCategory::OtherMultikills,
            EventCategory::GameFlow,
        ];
        EventCategory::ALL
            .into_iter()
            .filter(|category| !quiet.contains(category))
            .fold(CategorySet::NONE, |set, category| set.with(category, true))
    }

    pub fn contains(self, category: EventCategory) -> bool {
        self.0 & category.bit() != 0
    }

    pub fn with(self, category: EventCategory, shown: bool) -> CategorySet {
        if shown {
            CategorySet(self.0 | category.bit())
        } else {
            CategorySet(self.0 & !category.bit())
        }
    }

    pub fn toggled(self, category: EventCategory) -> CategorySet {
        self.with(category, !self.contains(category))
    }

    pub fn iter(self) -> impl Iterator<Item = EventCategory> {
        EventCategory::ALL
            .into_iter()
            .filter(move |category| self.contains(*category))
    }
}

impl Default for CategorySet {
    fn default() -> Self {
        CategorySet::recommended()
    }
}

impl Serialize for CategorySet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for CategorySet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let serde_json::Value::Array(items) = raw else {
            return Ok(CategorySet::default());
        };
        Ok(items
            .into_iter()
            .filter_map(|item| serde_json::from_value::<EventCategory>(item).ok())
            .fold(CategorySet::NONE, |set, category| set.with(category, true)))
    }
}

fn team_of(name: &str, info: &GameInfo) -> Option<Team> {
    info.player_named(name).map(|player| player.team)
}

fn kill_between_others(killer: &str, victim: &str, info: &GameInfo) -> EventCategory {
    let team = team_of(killer, info).or_else(|| {
        team_of(victim, info).map(|team| match team {
            Team::Order => Team::Chaos,
            Team::Chaos => Team::Order,
        })
    });
    if team == Some(info.me.team) {
        EventCategory::TeammateKills
    } else {
        EventCategory::EnemyKills
    }
}

pub fn category_of(kind: &GameEvent, info: &GameInfo) -> EventCategory {
    match kind {
        GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
        } => {
            if info.is_me(victim) {
                EventCategory::MyDeaths
            } else if info.is_me(killer) {
                EventCategory::MyKills
            } else if assisters.iter().any(|name| info.is_me(name)) {
                EventCategory::MyAssists
            } else {
                kill_between_others(killer, victim, info)
            }
        }
        GameEvent::Multikill { killer, .. } if info.is_me(killer) => EventCategory::MyMultikills,
        GameEvent::Multikill { .. } => EventCategory::OtherMultikills,
        GameEvent::Ace { .. } => EventCategory::Aces,
        GameEvent::DragonKill { .. } => EventCategory::Dragons,
        GameEvent::BaronKill { .. } => EventCategory::Barons,
        GameEvent::HeraldKill { .. } | GameEvent::HordeKill { .. } => EventCategory::Heralds,
        GameEvent::TurretKilled { .. } | GameEvent::InhibKilled { .. } => EventCategory::Structures,
        GameEvent::GameEnd { .. } => EventCategory::GameEnd,
        GameEvent::GameStart
        | GameEvent::MinionsSpawning
        | GameEvent::FirstBlood { .. }
        | GameEvent::FirstBrick { .. }
        | GameEvent::InhibRespawningSoon { .. }
        | GameEvent::InhibRespawned { .. }
        | GameEvent::Unknown { .. } => EventCategory::GameFlow,
    }
}

pub fn categorize(events: &[EventEntry], info: &GameInfo) -> Vec<EventCategory> {
    events
        .iter()
        .map(|event| category_of(&event.kind, info))
        .collect()
}

#[cfg(test)]
mod tests;
