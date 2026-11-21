use greyscreen_core::game::{GameEvent, GameResult, Player};
use greyscreen_core::library::EventEntry;
use greyscreen_engine::RecordingEntry;
use jiff::Zoned;

use super::format_size;

const RECENT_DAYS: i64 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Kda {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct When {
    pub day: String,
    pub time: String,
}

pub(crate) fn mode_label(mode: &str) -> String {
    match mode.to_ascii_uppercase().as_str() {
        "CLASSIC" => "Summoner's Rift".into(),
        "ARAM" => "ARAM".into(),
        "PRACTICETOOL" => "Practice Tool".into(),
        "CHERRY" => "Arena".into(),
        "URF" => "URF".into(),
        _ => title_case(mode),
    }
}

fn title_case(raw: &str) -> String {
    raw.split(|c: char| c == '_' || c.is_whitespace())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn when(at: &Zoned, now: &Zoned) -> When {
    let local = at.with_time_zone(now.time_zone().clone());
    let time = local.strftime("%H:%M").to_string();
    let days = now
        .date()
        .since(local.date())
        .map_or(-1, |span| i64::from(span.get_days()));
    let day = match days {
        0 => "Today".to_owned(),
        1 => "Yesterday".to_owned(),
        2..=RECENT_DAYS => local.strftime("%a").to_string(),
        _ if local.year() == now.year() => local.strftime("%b %-d").to_string(),
        _ => local.strftime("%b %-d, %Y").to_string(),
    };
    When { day, time }
}

fn same_player(player: &Player, name: &str) -> bool {
    let strip = |value: &str| {
        value
            .rsplit_once('#')
            .map_or(value.to_owned(), |(game, _)| game.to_owned())
    };
    let name = strip(name);
    strip(&player.name).eq_ignore_ascii_case(&name)
        || (player.is_bot && player.champion.eq_ignore_ascii_case(&name))
}

pub(crate) fn kda(events: &[EventEntry], me: &Player) -> Kda {
    let mut kda = Kda::default();
    for event in events {
        if let GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
        } = &event.kind
        {
            kda.kills += u32::from(same_player(me, killer));
            kda.deaths += u32::from(same_player(me, victim));
            kda.assists += u32::from(assisters.iter().any(|name| same_player(me, name)));
        }
    }
    kda
}

pub(crate) fn total_time(secs: f64) -> String {
    let minutes = (secs.max(0.0) / 60.0).round() as u64;
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, m) => format!("{h}h {m}m"),
    }
}

pub(crate) fn summary(recordings: &[RecordingEntry]) -> String {
    let count = |wanted| {
        recordings
            .iter()
            .filter(|r| r.meta.game.result == wanted)
            .count()
    };
    let games = recordings.len();
    let secs: f64 = recordings.iter().map(|r| r.meta.duration_secs).sum();
    let bytes: u64 = recordings.iter().map(|r| r.size_bytes).sum();
    format!(
        "{games} {} · {}W {}L · {} recorded · {}",
        if games == 1 { "game" } else { "games" },
        count(GameResult::Win),
        count(GameResult::Loss),
        total_time(secs),
        format_size(bytes),
    )
}

#[cfg(test)]
mod tests;
