use serde_json::Value;

use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team};

use crate::dto::{GameStats, PlayerEntry};

pub fn event_id(event: &Value) -> Option<u32> {
    event.get("EventID")?.as_u64()?.try_into().ok()
}

pub fn event_secs(event: &Value) -> Option<f64> {
    event.get("EventTime")?.as_f64()
}

pub fn loose_bool(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::String(s) => s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

pub fn strip_tag(name: &str) -> &str {
    name.rsplit_once('#')
        .map_or(name, |(game_name, _)| game_name)
}

fn text(event: &Value, key: &str) -> String {
    event
        .get(key)
        .and_then(Value::as_str)
        .map(|s| strip_tag(s).to_owned())
        .unwrap_or_default()
}

fn raw_text(event: &Value, key: &str) -> String {
    event
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn assisters(event: &Value) -> Vec<String> {
    event
        .get("Assisters")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(|s| strip_tag(s).to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn stolen(event: &Value) -> bool {
    event.get("Stolen").is_some_and(loose_bool)
}

fn team_from(name: &str) -> Option<Team> {
    match name.to_ascii_uppercase().as_str() {
        "ORDER" => Some(Team::Order),
        "CHAOS" => Some(Team::Chaos),
        _ => None,
    }
}

fn result_from(name: &str) -> GameResult {
    match name.to_ascii_lowercase().as_str() {
        "win" => GameResult::Win,
        "lose" | "loss" | "defeat" => GameResult::Loss,
        _ => GameResult::Unknown,
    }
}

fn unknown(name: &str) -> GameEvent {
    GameEvent::Unknown {
        name: name.to_owned(),
    }
}

pub fn to_game_event(event: &Value) -> GameEvent {
    let name = event
        .get("EventName")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match name {
        "GameStart" => GameEvent::GameStart,
        "MinionsSpawning" => GameEvent::MinionsSpawning,
        "FirstBlood" => GameEvent::FirstBlood {
            recipient: text(event, "Recipient"),
        },
        "ChampionKill" => GameEvent::ChampionKill {
            killer: text(event, "KillerName"),
            victim: text(event, "VictimName"),
            assisters: assisters(event),
        },
        "Multikill" => match event.get("KillStreak").and_then(Value::as_u64) {
            Some(streak) => GameEvent::Multikill {
                killer: text(event, "KillerName"),
                streak: u8::try_from(streak).unwrap_or(u8::MAX),
            },
            None => unknown(name),
        },
        "Ace" => match event
            .get("AcingTeam")
            .and_then(Value::as_str)
            .and_then(team_from)
        {
            Some(acing_team) => GameEvent::Ace {
                acer: text(event, "Acer"),
                acing_team,
            },
            None => unknown(name),
        },
        "FirstBrick" => GameEvent::FirstBrick {
            killer: text(event, "KillerName"),
        },
        "TurretKilled" => GameEvent::TurretKilled {
            turret: raw_text(event, "TurretKilled"),
            killer: text(event, "KillerName"),
            assisters: assisters(event),
        },
        "InhibKilled" => GameEvent::InhibKilled {
            inhib: raw_text(event, "InhibKilled"),
            killer: text(event, "KillerName"),
            assisters: assisters(event),
        },
        "InhibRespawningSoon" => GameEvent::InhibRespawningSoon {
            inhib: raw_text(event, "InhibRespawningSoon"),
        },
        "InhibRespawned" => GameEvent::InhibRespawned {
            inhib: raw_text(event, "InhibRespawned"),
        },
        "DragonKill" => GameEvent::DragonKill {
            dragon_type: raw_text(event, "DragonType"),
            stolen: stolen(event),
            killer: text(event, "KillerName"),
            assisters: assisters(event),
        },
        "HeraldKill" => GameEvent::HeraldKill {
            stolen: stolen(event),
            killer: text(event, "KillerName"),
            assisters: assisters(event),
        },
        "BaronKill" => GameEvent::BaronKill {
            stolen: stolen(event),
            killer: text(event, "KillerName"),
            assisters: assisters(event),
        },
        "HordeKill" => GameEvent::HordeKill {
            killer: text(event, "KillerName"),
            assisters: assisters(event),
        },
        "GameEnd" => GameEvent::GameEnd {
            result: result_from(&raw_text(event, "Result")),
        },
        other => unknown(other),
    }
}

fn to_player(entry: &PlayerEntry) -> Option<Player> {
    let name = if entry.riot_id_game_name.is_empty() {
        strip_tag(&entry.summoner_name)
    } else {
        &entry.riot_id_game_name
    };
    Some(Player {
        name: name.to_owned(),
        champion: entry.champion_name.clone(),
        team: team_from(&entry.team)?,
        is_bot: entry.is_bot,
    })
}

fn find_me<'a>(active_name: &str, players: &'a [PlayerEntry]) -> Option<&'a PlayerEntry> {
    let game_name = strip_tag(active_name);
    players
        .iter()
        .find(|p| p.riot_id.eq_ignore_ascii_case(active_name))
        .or_else(|| {
            players
                .iter()
                .find(|p| p.riot_id_game_name.eq_ignore_ascii_case(game_name))
        })
        .or_else(|| {
            players
                .iter()
                .find(|p| strip_tag(&p.summoner_name).eq_ignore_ascii_case(game_name))
        })
}

pub fn to_game_info(
    active_name: &str,
    players: &[PlayerEntry],
    stats: &GameStats,
) -> Option<GameInfo> {
    let me = to_player(find_me(active_name, players)?)?;
    Some(GameInfo {
        mode: stats.game_mode.clone(),
        map: stats.map_number,
        me,
        players: players.iter().filter_map(to_player).collect(),
    })
}

#[cfg(test)]
mod tests;
