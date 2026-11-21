use crate::game::{GameEvent, GameInfo, GameResult, Player, Team};
use crate::settings::ClipSettings;

mod display;

pub use display::{LabelDisplay, Tone, describe_label, dominant_label};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Objective {
    Dragon(String),
    Herald,
    Baron,
    VoidGrub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Structure {
    Turret,
    Inhibitor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Highlight {
    MyKill,
    MyDeath,
    MyAssist,
    MyMultikill { streak: u8 },
    MyFirstBlood,
    AllyKill,
    EnemyKill,
    TeamObjective(Objective),
    EnemyObjective(Objective),
    TeamStructure(Structure),
    EnemyStructure(Structure),
    MyAce,
    EnemyAce,
    GameWon,
    GameLost,
}

impl Highlight {
    pub fn label(&self) -> &'static str {
        match self {
            Highlight::MyKill => "MyKill",
            Highlight::MyDeath => "MyDeath",
            Highlight::MyAssist => "MyAssist",
            Highlight::MyMultikill { streak } => match streak {
                2 => "Multikill2",
                3 => "Multikill3",
                4 => "Multikill4",
                5 => "Multikill5",
                _ => "Multikill",
            },
            Highlight::MyFirstBlood => "FirstBlood",
            Highlight::AllyKill => "AllyKill",
            Highlight::EnemyKill => "EnemyKill",
            Highlight::TeamObjective(objective) => match objective {
                Objective::Dragon(_) => "Dragon",
                Objective::Herald => "Herald",
                Objective::Baron => "Baron",
                Objective::VoidGrub => "VoidGrub",
            },
            Highlight::EnemyObjective(objective) => match objective {
                Objective::Dragon(_) => "EnemyDragon",
                Objective::Herald => "EnemyHerald",
                Objective::Baron => "EnemyBaron",
                Objective::VoidGrub => "EnemyVoidGrub",
            },
            Highlight::TeamStructure(structure) => match structure {
                Structure::Turret => "Turret",
                Structure::Inhibitor => "Inhibitor",
            },
            Highlight::EnemyStructure(structure) => match structure {
                Structure::Turret => "EnemyTurret",
                Structure::Inhibitor => "EnemyInhibitor",
            },
            Highlight::MyAce => "Ace",
            Highlight::EnemyAce => "EnemyAce",
            Highlight::GameWon => "Victory",
            Highlight::GameLost => "Defeat",
        }
    }

    pub fn is_clip_worthy(&self, clips: &ClipSettings) -> bool {
        match self {
            Highlight::MyKill | Highlight::MyFirstBlood => clips.kills,
            Highlight::MyDeath => clips.deaths,
            Highlight::MyAssist => clips.assists,
            Highlight::MyMultikill { .. } => clips.multikills,
            Highlight::TeamObjective(_) | Highlight::EnemyObjective(_) => clips.objectives,
            Highlight::TeamStructure(_) | Highlight::EnemyStructure(_) => clips.structures,
            Highlight::MyAce => clips.aces,
            Highlight::AllyKill
            | Highlight::EnemyKill
            | Highlight::EnemyAce
            | Highlight::GameWon
            | Highlight::GameLost => false,
        }
    }
}

pub fn structure_team(id: &str) -> Option<Team> {
    id.split('_').find_map(|part| match part {
        "T1" => Some(Team::Order),
        "T2" => Some(Team::Chaos),
        _ => None,
    })
}

fn strip_tag(name: &str) -> &str {
    name.rsplit_once('#')
        .map_or(name, |(game_name, _)| game_name)
}

fn names_match(player: &Player, name: &str) -> bool {
    let name = strip_tag(name);
    strip_tag(&player.name).eq_ignore_ascii_case(name)
        || (player.is_bot && player.champion.eq_ignore_ascii_case(name))
}

fn team_of(info: &GameInfo, name: &str) -> Option<Team> {
    if names_match(&info.me, name) {
        return Some(info.me.team);
    }
    info.players
        .iter()
        .find(|player| names_match(player, name))
        .map(|player| player.team)
}

fn objective_highlight(info: &GameInfo, killer: &str, objective: Objective) -> Vec<Highlight> {
    match team_of(info, killer) {
        Some(team) if team == info.me.team => vec![Highlight::TeamObjective(objective)],
        Some(_) => vec![Highlight::EnemyObjective(objective)],
        None => Vec::new(),
    }
}

fn structure_highlight(info: &GameInfo, id: &str, structure: Structure) -> Vec<Highlight> {
    match structure_team(id) {
        Some(team) if team == info.me.team => vec![Highlight::EnemyStructure(structure)],
        Some(_) => vec![Highlight::TeamStructure(structure)],
        None => Vec::new(),
    }
}

fn classify_kill(
    info: &GameInfo,
    killer: &str,
    victim: &str,
    assisters: &[String],
) -> Vec<Highlight> {
    let mut highlights = Vec::new();
    if names_match(&info.me, killer) {
        highlights.push(Highlight::MyKill);
    }
    if names_match(&info.me, victim) {
        highlights.push(Highlight::MyDeath);
    }
    if assisters.iter().any(|name| names_match(&info.me, name)) {
        highlights.push(Highlight::MyAssist);
    }
    if highlights.is_empty() {
        match team_of(info, killer) {
            Some(team) if team == info.me.team => highlights.push(Highlight::AllyKill),
            Some(_) => highlights.push(Highlight::EnemyKill),
            None => {}
        }
    }
    highlights
}

pub fn classify(event: &GameEvent, info: &GameInfo) -> Vec<Highlight> {
    match event {
        GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
        } => classify_kill(info, killer, victim, assisters),
        GameEvent::FirstBlood { recipient } if names_match(&info.me, recipient) => {
            vec![Highlight::MyFirstBlood]
        }
        GameEvent::Multikill { killer, streak } if names_match(&info.me, killer) => {
            vec![Highlight::MyMultikill { streak: *streak }]
        }
        GameEvent::DragonKill {
            dragon_type,
            killer,
            ..
        } => objective_highlight(info, killer, Objective::Dragon(dragon_type.clone())),
        GameEvent::HeraldKill { killer, .. } => {
            objective_highlight(info, killer, Objective::Herald)
        }
        GameEvent::BaronKill { killer, .. } => objective_highlight(info, killer, Objective::Baron),
        GameEvent::HordeKill { killer, .. } => {
            objective_highlight(info, killer, Objective::VoidGrub)
        }
        GameEvent::TurretKilled { turret, .. } => {
            structure_highlight(info, turret, Structure::Turret)
        }
        GameEvent::InhibKilled { inhib, .. } => {
            structure_highlight(info, inhib, Structure::Inhibitor)
        }
        GameEvent::Ace { acing_team, .. } if *acing_team == info.me.team => {
            vec![Highlight::MyAce]
        }
        GameEvent::Ace { .. } => vec![Highlight::EnemyAce],
        GameEvent::GameEnd {
            result: GameResult::Win,
        } => vec![Highlight::GameWon],
        GameEvent::GameEnd {
            result: GameResult::Loss,
        } => vec![Highlight::GameLost],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
