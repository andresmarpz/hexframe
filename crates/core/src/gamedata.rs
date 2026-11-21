use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, OnceLock, PoisonError};

use serde::Deserialize;

use crate::game::{GameEvent, GameInfo, Team};
use crate::highlight::structure_team;

#[derive(Debug, Default, Deserialize)]
struct Table {
    modes: Vec<ModeRow>,
    maps: Vec<MapRow>,
}

#[derive(Debug, Deserialize)]
struct ModeRow {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct MapRow {
    id: u32,
    name: String,
    #[serde(default)]
    turret_tiers: BTreeMap<u8, String>,
}

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../data/game_data.json")).unwrap_or_else(|err| {
            tracing::error!(%err, "the bundled game data is unreadable");
            Table::default()
        })
    })
}

pub fn mode_name(mode: &str) -> Option<&'static str> {
    table()
        .modes
        .iter()
        .find(|row| row.id.eq_ignore_ascii_case(mode))
        .map(|row| row.name.as_str())
}

pub fn map_name(id: u32) -> Option<&'static str> {
    map_row(Some(id)).map(|row| row.name.as_str())
}

fn map_row(id: Option<u32>) -> Option<&'static MapRow> {
    let id = id?;
    table().maps.iter().find(|row| row.id == id)
}

fn side(team: Team) -> &'static str {
    match team {
        Team::Order => "Blue",
        Team::Chaos => "Red",
    }
}

fn lane(code: &str) -> Option<&'static str> {
    match code {
        "L" => Some("top"),
        "C" => Some("mid"),
        "R" => Some("bot"),
        _ => None,
    }
}

fn classic_tier(lane_code: &str, number: &str) -> Option<&'static str> {
    let n: u8 = number.parse().ok()?;
    match (lane_code, n) {
        ("C", 5) | ("L" | "R", 3) => Some("outer turret"),
        ("C", 4) | ("L" | "R", 2) => Some("inner turret"),
        ("C", 3) | ("L" | "R", 1) => Some("inhibitor turret"),
        ("C", 1 | 2) => Some("nexus turret"),
        _ => None,
    }
}

struct Decoded {
    name: String,
    exact: bool,
}

fn exact(name: String) -> Option<Decoded> {
    Some(Decoded { name, exact: true })
}

fn decode_classic(parts: &[&str], team: Team) -> Option<Decoded> {
    let side = side(team);
    match parts {
        ["Turret", _, "C", number @ ("01" | "02"), ..] => {
            exact(format!("{side} {}", classic_tier("C", number)?))
        }
        ["Turret", _, lane_code, number, ..] => exact(format!(
            "{side} {} {}",
            lane(lane_code)?,
            classic_tier(lane_code, number)?
        )),
        ["Barracks", _, code] => exact(format!("{side} {} inhibitor", lane(code.get(..1)?)?)),
        _ => None,
    }
}

fn decode_modern(parts: &[&str], team: Team, map: Option<u32>) -> Option<Decoded> {
    let [kind, _, _lane, position, ..] = parts else {
        return None;
    };
    let position: u8 = position.strip_prefix('P')?.parse().ok()?;
    let side = side(team);
    match *kind {
        "Inhib" => exact(format!("{side} inhibitor")),
        "Turret" => match map_row(map).and_then(|row| row.turret_tiers.get(&position)) {
            Some(tier) => exact(format!("{side} {tier} turret")),
            None => Some(Decoded {
                name: format!("{side} turret"),
                exact: false,
            }),
        },
        _ => None,
    }
}

fn decode(id: &str, map: Option<u32>) -> Option<Decoded> {
    let team = structure_team(id)?;
    let parts: Vec<&str> = id.split('_').collect();
    let modern = parts
        .get(1)
        .is_some_and(|part| part.starts_with("TOrder") || part.starts_with("TChaos"));
    if modern {
        decode_modern(&parts, team, map)
    } else {
        decode_classic(&parts, team)
    }
}

pub fn is_structure_id(name: &str) -> bool {
    ["Turret_", "Inhib_", "Barracks_"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

pub fn structure_name(id: &str, map: Option<u32>) -> Option<String> {
    decode(id, map).map(|decoded| decoded.name)
}

pub fn structure_label(id: &str, map: Option<u32>) -> String {
    structure_name(id, map).unwrap_or_else(|| {
        let kind = match id.split('_').next() {
            Some("Turret") => "Turret",
            Some("Inhib" | "Barracks") => "Inhibitor",
            _ => "Structure",
        };
        match structure_team(id) {
            Some(team) => format!("{} {}", side(team), kind.to_ascii_lowercase()),
            None => kind.to_owned(),
        }
    })
}

fn first_time(key: String) -> bool {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SEEN.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key)
}

pub fn report_unmapped_game(info: &GameInfo) {
    if mode_name(&info.mode).is_none() && first_time(format!("mode:{}", info.mode)) {
        tracing::warn!(mode = %info.mode, map = ?info.map, "game mode missing from the bundled game data");
    }
    if let Some(map) = info.map
        && map_name(map).is_none()
        && first_time(format!("map:{map}"))
    {
        tracing::warn!(map, mode = %info.mode, "map missing from the bundled game data");
    }
}

fn shape(id: &str) -> String {
    id.split('_')
        .filter(|part| !part.chars().all(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join("_")
}

pub fn report_unmapped_event(kind: &GameEvent, info: &GameInfo) {
    let (GameEvent::TurretKilled { turret: id, .. } | GameEvent::InhibKilled { inhib: id, .. }) =
        kind
    else {
        return;
    };
    let mapped = decode(id, info.map).is_some_and(|decoded| decoded.exact);
    if !mapped && first_time(format!("structure:{:?}:{}", info.map, shape(id))) {
        tracing::warn!(id, map = ?info.map, mode = %info.mode, "structure id missing from the bundled game data");
    }
}

#[cfg(test)]
mod tests;
