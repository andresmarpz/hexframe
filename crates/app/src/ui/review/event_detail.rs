use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team};
use greyscreen_core::highlight::structure_team;
use greyscreen_core::library::EventEntry;

use crate::ui::format_clock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Span {
    Strong(String),
    Muted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub lead: String,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Detail {
    pub title: Vec<Span>,
    pub time: String,
    pub lines: Vec<Line>,
}

fn strong(text: impl Into<String>) -> Span {
    Span::Strong(text.into())
}

fn muted(text: impl Into<String>) -> Span {
    Span::Muted(text.into())
}

fn line(lead: &str, spans: Vec<Span>) -> Line {
    Line {
        lead: lead.to_owned(),
        spans,
    }
}

fn strip_tag(name: &str) -> &str {
    name.rsplit_once('#').map_or(name, |(game, _)| game)
}

fn same_name(a: &str, b: &str) -> bool {
    strip_tag(a).eq_ignore_ascii_case(strip_tag(b))
}

fn is_player(player: &Player, name: &str) -> bool {
    same_name(&player.name, name) || (player.is_bot && player.champion.eq_ignore_ascii_case(name))
}

fn player_named<'a>(info: &'a GameInfo, name: &str) -> Option<&'a Player> {
    info.players.iter().find(|player| is_player(player, name))
}

fn person(info: &GameInfo, name: &str) -> Vec<Span> {
    if is_player(&info.me, name) {
        return vec![strong("You")];
    }
    if let Some(player) = player_named(info, name) {
        let mut spans = vec![strong(player.champion.clone())];
        let name = strip_tag(&player.name);
        if !player.is_bot && !name.is_empty() {
            spans.push(muted(name));
        }
        return spans;
    }
    if name.starts_with("Minion_") {
        return vec![strong("Minions")];
    }
    if let Some(structure) = structure_name(name) {
        return vec![strong(structure)];
    }
    vec![strong(name)]
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

fn turret_tier(lane_code: &str, number: &str) -> Option<&'static str> {
    let n: u8 = number.parse().ok()?;
    match (lane_code, n) {
        ("C", 5) | ("L" | "R", 3) => Some("outer turret"),
        ("C", 4) | ("L" | "R", 2) => Some("inner turret"),
        ("C", 3) | ("L" | "R", 1) => Some("inhibitor turret"),
        ("C", 1 | 2) => Some("nexus turret"),
        _ => None,
    }
}

pub(crate) fn structure_name(id: &str) -> Option<String> {
    let team = structure_team(id)?;
    let parts: Vec<&str> = id.split('_').collect();
    match parts.as_slice() {
        ["Turret", _, "C", number @ ("01" | "02"), ..] => {
            turret_tier("C", number).map(|tier| format!("{} {tier}", side(team)))
        }
        ["Turret", _, lane_code, number, ..] => {
            let place = lane(lane_code)?;
            let tier = turret_tier(lane_code, number)?;
            Some(format!("{} {place} {tier}", side(team)))
        }
        ["Barracks", _, code] => {
            let place = lane(code.get(..1)?)?;
            Some(format!("{} {place} inhibitor", side(team)))
        }
        _ => None,
    }
}

fn structure_label(id: &str) -> String {
    structure_name(id).unwrap_or_else(|| id.to_owned())
}

fn assist_lines(info: &GameInfo, assisters: &[String]) -> Vec<Line> {
    assisters
        .iter()
        .map(|name| line("Assist", person(info, name)))
        .collect()
}

fn credit(info: &GameInfo, lead: &str, killer: &str, assisters: &[String]) -> Vec<Line> {
    let mut lines = vec![line(lead, person(info, killer))];
    lines.extend(assist_lines(info, assisters));
    lines
}

fn streak_title(streak: u8) -> &'static str {
    match streak {
        2 => "Double kill",
        3 => "Triple kill",
        4 => "Quadra kill",
        5 => "Pentakill",
        _ => "Multikill",
    }
}

pub(crate) fn streak_kills<'a>(
    events: &'a [EventEntry],
    at: &EventEntry,
    killer: &str,
    streak: u8,
) -> Vec<&'a EventEntry> {
    let mut kills: Vec<&EventEntry> = events
        .iter()
        .filter(|e| {
            e.rec_secs <= at.rec_secs
                && matches!(&e.kind, GameEvent::ChampionKill { killer: k, .. } if same_name(k, killer))
        })
        .collect();
    kills.sort_by(|a, b| a.rec_secs.total_cmp(&b.rec_secs).then(a.id.cmp(&b.id)));
    let skip = kills.len().saturating_sub(usize::from(streak));
    kills.split_off(skip)
}

fn dragon_title(kind: &str) -> String {
    if kind.is_empty() {
        "Dragon".to_owned()
    } else {
        format!("{kind} dragon")
    }
}

fn objective(
    title: String,
    stolen: bool,
    killer: &str,
    assisters: &[String],
    info: &GameInfo,
) -> (Vec<Span>, Vec<Line>) {
    let mut lines = credit(info, "Slain by", killer, assisters);
    if stolen {
        lines.insert(0, line("", vec![strong("Stolen")]));
    }
    (vec![strong(title)], lines)
}

fn structure_credit(
    id: &str,
    killer: &str,
    assisters: &[String],
    info: &GameInfo,
) -> (Vec<Span>, Vec<Line>) {
    (
        vec![strong(format!("{} destroyed", structure_label(id)))],
        credit(info, "Killed by", killer, assisters),
    )
}

pub(crate) fn describe(
    event: &EventEntry,
    events: &[EventEntry],
    info: &GameInfo,
) -> Option<Detail> {
    let (title, lines) = match &event.kind {
        GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
        } => {
            let mut title = person(info, killer);
            title.push(muted("killed"));
            title.extend(person(info, victim));
            (title, assist_lines(info, assisters))
        }
        GameEvent::Multikill { killer, streak } => {
            let mut title = vec![strong(streak_title(*streak)), muted("by")];
            title.extend(person(info, killer));
            let rows = streak_kills(events, event, killer, *streak)
                .into_iter()
                .filter_map(|kill| match &kill.kind {
                    GameEvent::ChampionKill { victim, .. } => Some(line(
                        &format_clock(kill.rec_secs as u32),
                        person(info, victim),
                    )),
                    _ => None,
                })
                .collect();
            (title, rows)
        }
        GameEvent::TurretKilled {
            turret,
            killer,
            assisters,
        } => structure_credit(turret, killer, assisters, info),
        GameEvent::InhibKilled {
            inhib,
            killer,
            assisters,
        } => structure_credit(inhib, killer, assisters, info),
        GameEvent::InhibRespawningSoon { inhib } => (
            vec![strong(format!(
                "{} respawning soon",
                structure_label(inhib)
            ))],
            Vec::new(),
        ),
        GameEvent::InhibRespawned { inhib } => (
            vec![strong(format!("{} respawned", structure_label(inhib)))],
            Vec::new(),
        ),
        GameEvent::DragonKill {
            dragon_type,
            stolen,
            killer,
            assisters,
        } => objective(dragon_title(dragon_type), *stolen, killer, assisters, info),
        GameEvent::HeraldKill {
            stolen,
            killer,
            assisters,
        } => objective("Rift Herald".into(), *stolen, killer, assisters, info),
        GameEvent::BaronKill {
            stolen,
            killer,
            assisters,
        } => objective("Baron Nashor".into(), *stolen, killer, assisters, info),
        GameEvent::HordeKill { killer, assisters } => {
            objective("Void Grub".into(), false, killer, assisters, info)
        }
        GameEvent::FirstBlood { recipient } => (
            vec![strong("First blood")],
            vec![line("Taken by", person(info, recipient))],
        ),
        GameEvent::FirstBrick { killer } => (
            vec![strong("First tower")],
            vec![line("Taken by", person(info, killer))],
        ),
        GameEvent::Ace { acer, acing_team } => (
            vec![strong("Ace"), muted(format!("{} team", side(*acing_team)))],
            vec![line("Acer", person(info, acer))],
        ),
        GameEvent::GameEnd { result } => {
            let text = match result {
                GameResult::Win => "Victory",
                GameResult::Loss => "Defeat",
                GameResult::Unknown => "Game over",
            };
            (vec![strong(text)], Vec::new())
        }
        GameEvent::GameStart => (vec![strong("Game start")], Vec::new()),
        GameEvent::MinionsSpawning => (vec![strong("Minions spawning")], Vec::new()),
        GameEvent::Unknown { .. } => return None,
    };
    Some(Detail {
        title,
        time: format_clock(event.rec_secs as u32),
        lines,
    })
}

#[cfg(test)]
mod tests;
