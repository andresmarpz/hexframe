use greyscreen_core::game::{GameEvent, GameInfo, GameResult};
use greyscreen_core::gamedata::mode_name;
use greyscreen_core::highlight::describe_label;
use greyscreen_core::library::{ClipEntry, EventEntry};
use greyscreen_core::review::deaths;
use greyscreen_engine::RecordingEntry;
use jiff::Zoned;

use crate::ui::format_size;

const RECENT_DAYS: i64 = 6;
const WEEK_SECS: i64 = 7 * 24 * 3600;

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

impl When {
    pub(crate) fn label(&self) -> String {
        format!("{} {}", self.day, self.time)
    }
}

pub(crate) fn mode_label(mode: &str) -> String {
    mode_name(mode).map_or_else(|| title_case(mode), str::to_owned)
}

pub(crate) fn short_mode_label(mode: &str) -> String {
    let full = mode_label(mode);
    if full == "Summoner's Rift" {
        "Rift".to_owned()
    } else {
        full
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

pub(crate) fn day_heading(at: &Zoned, now: &Zoned) -> String {
    let local = at.with_time_zone(now.time_zone().clone());
    let days = now
        .date()
        .since(local.date())
        .map_or(-1, |span| i64::from(span.get_days()));
    match days {
        0 => "Today".to_owned(),
        1 => "Yesterday".to_owned(),
        2..=RECENT_DAYS => local.strftime("%A").to_string(),
        _ if local.year() == now.year() => local.strftime("%B %-d").to_string(),
        _ => local.strftime("%B %-d, %Y").to_string(),
    }
}

pub(crate) fn kda(events: &[EventEntry], info: &GameInfo) -> Kda {
    let mut kda = Kda::default();
    for event in events {
        if let GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
        } = &event.kind
        {
            kda.kills += u32::from(info.is_me(killer));
            kda.deaths += u32::from(info.is_me(victim));
            kda.assists += u32::from(assisters.iter().any(|name| info.is_me(name)));
        }
    }
    kda
}

pub(crate) fn death_total(rec: &RecordingEntry) -> usize {
    let meta = &rec.meta;
    deaths(&meta.events, &meta.game.info).len()
}

pub(crate) fn review_progress(rec: &RecordingEntry) -> Option<String> {
    let meta = &rec.meta;
    let list = deaths(&meta.events, &meta.game.info);
    (!list.is_empty()).then(|| format!("{}/{}", meta.review.reviewed_among(&list), list.len()))
}

pub(crate) fn death_hint(rec: &RecordingEntry) -> Option<String> {
    let meta = &rec.meta;
    let list = deaths(&meta.events, &meta.game.info);
    if list.is_empty() {
        return None;
    }
    let total = list.len();
    let noun = if total == 1 { "death" } else { "deaths" };
    Some(match meta.review.reviewed_among(&list) {
        0 => format!("{total} {noun}"),
        reviewed => format!("{total} {noun} · {reviewed} reviewed"),
    })
}

pub(crate) fn total_time(secs: f64) -> String {
    let minutes = (secs.max(0.0) / 60.0).round() as u64;
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, m) => format!("{h}h {m}m"),
    }
}

pub(crate) fn summary(recordings: &[&RecordingEntry], total: usize) -> String {
    let count = |wanted| {
        recordings
            .iter()
            .filter(|r| r.meta.game.result == wanted)
            .count()
    };
    let games = recordings.len();
    let secs: f64 = recordings.iter().map(|r| r.meta.duration_secs).sum();
    let bytes: u64 = recordings.iter().map(|r| r.size_bytes).sum();
    let games_label = if games == total {
        games.to_string()
    } else {
        format!("{games} of {total}")
    };
    format!(
        "{games_label} {} · {}W {}L · {} recorded · {}",
        if total == 1 { "game" } else { "games" },
        count(GameResult::Win),
        count(GameResult::Loss),
        total_time(secs),
        format_size(bytes),
    )
}

pub(crate) fn humanize(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

pub(crate) fn clip_title(label: &str) -> String {
    let known = describe_label(label);
    if known.title == describe_label("").title {
        humanize(label)
    } else {
        known.title.to_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct WeekStats {
    pub games: usize,
    pub wins: usize,
    pub losses: usize,
    pub secs: f64,
}

impl WeekStats {
    pub(crate) fn win_rate(&self) -> Option<u32> {
        let decided = self.wins + self.losses;
        (decided > 0).then(|| (self.wins as f64 * 100.0 / decided as f64).round() as u32)
    }
}

fn this_week<'a>(
    recordings: &'a [RecordingEntry],
    now: &Zoned,
) -> impl Iterator<Item = &'a RecordingEntry> {
    let cutoff = now.timestamp().as_second() - WEEK_SECS;
    recordings
        .iter()
        .filter(move |rec| rec.meta.started_at.timestamp().as_second() >= cutoff)
}

pub(crate) fn week_stats(recordings: &[RecordingEntry], now: &Zoned) -> WeekStats {
    let mut stats = WeekStats::default();
    for rec in this_week(recordings, now) {
        stats.games += 1;
        stats.secs += rec.meta.duration_secs;
        match rec.meta.game.result {
            GameResult::Win => stats.wins += 1,
            GameResult::Loss => stats.losses += 1,
            GameResult::Unknown => {}
        }
    }
    stats
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WeekStrip {
    pub results: Vec<GameResult>,
    pub most_played: Option<String>,
}

#[allow(dead_code)]
pub(crate) fn week_strip(recordings: &[RecordingEntry], now: &Zoned, limit: usize) -> WeekStrip {
    let mut games: Vec<&RecordingEntry> = this_week(recordings, now).collect();
    games.sort_by_key(|rec| rec.meta.started_at.timestamp());
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for rec in &games {
        let champion = rec.meta.game.info.me.champion.as_str();
        match counts.iter_mut().find(|(name, _)| *name == champion) {
            Some((_, count)) => *count += 1,
            None => counts.push((champion, 1)),
        }
    }
    let most_played = counts
        .iter()
        .enumerate()
        .max_by_key(|(order, (_, count))| (*count, *order))
        .map(|(_, (name, _))| (*name).to_owned());
    let skip = games.len().saturating_sub(limit);
    WeekStrip {
        results: games
            .iter()
            .skip(skip)
            .map(|rec| rec.meta.game.result)
            .collect(),
        most_played,
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RecentClip<'a> {
    pub entry: &'a RecordingEntry,
    pub index: usize,
    pub clip: &'a ClipEntry,
}

pub(crate) fn recent_clips(recordings: &[RecordingEntry], limit: usize) -> Vec<RecentClip<'_>> {
    let mut clips: Vec<(f64, RecentClip<'_>)> = recordings
        .iter()
        .flat_map(|entry| {
            let start = entry.meta.started_at.timestamp().as_second() as f64;
            entry
                .meta
                .clips
                .iter()
                .enumerate()
                .map(move |(index, clip)| {
                    (start + clip.start_secs, RecentClip { entry, index, clip })
                })
        })
        .collect();
    clips.sort_by(|a, b| b.0.total_cmp(&a.0));
    clips
        .into_iter()
        .take(limit)
        .map(|(_, clip)| clip)
        .collect()
}

pub(crate) fn game_secs_at(entry: &RecordingEntry, rec_secs: f64) -> f64 {
    let offset = entry
        .meta
        .events
        .first()
        .map_or(0.0, |event| event.rec_secs - event.game_secs);
    (rec_secs - offset).max(0.0)
}
