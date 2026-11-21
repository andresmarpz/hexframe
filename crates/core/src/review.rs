use serde::{Deserialize, Serialize};

use crate::game::{GameEvent, Player};
use crate::library::EventEntry;

pub(crate) const WINDOW_BEFORE_SECS: f64 = 12.0;
pub(crate) const WINDOW_AFTER_SECS: f64 = 3.0;
pub(crate) const SLOW_MO_TAIL_SECS: f64 = 4.0;
pub const SLOW_MO_RATE: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathTag {
    OutOfPosition,
    Overextended,
    NoVision,
    BadTrade,
    Ganked,
    Teamfight,
    Unavoidable,
}

impl DeathTag {
    pub const ALL: [DeathTag; 7] = [
        DeathTag::OutOfPosition,
        DeathTag::Overextended,
        DeathTag::NoVision,
        DeathTag::BadTrade,
        DeathTag::Ganked,
        DeathTag::Teamfight,
        DeathTag::Unavoidable,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            DeathTag::OutOfPosition => "Out of position",
            DeathTag::Overextended => "Overextended",
            DeathTag::NoVision => "No vision",
            DeathTag::BadTrade => "Bad trade",
            DeathTag::Ganked => "Ganked",
            DeathTag::Teamfight => "Teamfight",
            DeathTag::Unavoidable => "Unavoidable",
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            DeathTag::OutOfPosition => "out-of-position",
            DeathTag::Overextended => "overextended",
            DeathTag::NoVision => "no-vision",
            DeathTag::BadTrade => "bad-trade",
            DeathTag::Ganked => "ganked",
            DeathTag::Teamfight => "teamfight",
            DeathTag::Unavoidable => "unavoidable",
        }
    }

    fn order(self) -> usize {
        Self::ALL
            .iter()
            .position(|t| *t == self)
            .unwrap_or(usize::MAX)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Death {
    pub event_id: u32,
    pub game_secs: f64,
    pub rec_secs: f64,
    pub killer: String,
    pub assisters: Vec<String>,
}

impl Death {
    pub fn window(&self, duration: f64) -> (f64, f64) {
        let start = (self.rec_secs - WINDOW_BEFORE_SECS).max(0.0);
        let mut end = self.rec_secs + WINDOW_AFTER_SECS;
        if duration > 0.0 {
            end = end.min(duration);
        }
        (start, end.max(start))
    }

    pub fn slow_mo_from(&self, duration: f64) -> f64 {
        let (start, end) = self.window(duration);
        (end - SLOW_MO_TAIL_SECS).max(start)
    }
}

pub fn deaths(events: &[EventEntry], me: &Player) -> Vec<Death> {
    let mut found: Vec<Death> = events
        .iter()
        .filter_map(|event| match &event.kind {
            GameEvent::ChampionKill {
                killer,
                victim,
                assisters,
            } if me.answers_to(victim) => Some(Death {
                event_id: event.id,
                game_secs: event.game_secs,
                rec_secs: event.rec_secs,
                killer: killer.clone(),
                assisters: assisters.clone(),
            }),
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| a.rec_secs.total_cmp(&b.rec_secs));
    found
}

pub fn killer_label(name: &str, players: &[Player]) -> String {
    if let Some(player) = players.iter().find(|p| p.answers_to(name)) {
        return player.champion.clone();
    }
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("minion") {
        "a minion".into()
    } else if lower.starts_with("turret") {
        "a turret".into()
    } else if lower.starts_with("sru_") || lower.starts_with("monster") {
        "a jungle monster".into()
    } else if name.trim().is_empty() {
        "the environment".into()
    } else {
        name.to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeathReview {
    pub event_id: u32,
    pub tags: Vec<DeathTag>,
    pub note: String,
}

impl DeathReview {
    fn is_blank(&self) -> bool {
        self.tags.is_empty() && self.note.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewData {
    pub deaths: Vec<DeathReview>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSummary {
    pub deaths: usize,
    pub reviewed: usize,
    pub notes: usize,
    pub tags: Vec<(DeathTag, usize)>,
}

impl ReviewData {
    pub fn is_empty(&self) -> bool {
        self.deaths.is_empty()
    }

    pub fn get(&self, event_id: u32) -> Option<&DeathReview> {
        self.deaths.iter().find(|d| d.event_id == event_id)
    }

    pub fn has_tag(&self, event_id: u32, tag: DeathTag) -> bool {
        self.get(event_id).is_some_and(|d| d.tags.contains(&tag))
    }

    pub fn note(&self, event_id: u32) -> &str {
        self.get(event_id).map_or("", |d| d.note.as_str())
    }

    pub fn is_reviewed(&self, event_id: u32) -> bool {
        self.get(event_id).is_some()
    }

    pub fn toggle_tag(&mut self, event_id: u32, tag: DeathTag) -> bool {
        let entry = self.entry(event_id);
        let on = !entry.tags.contains(&tag);
        if on {
            entry.tags.push(tag);
            entry.tags.sort_by_key(|t| t.order());
        } else {
            entry.tags.retain(|t| *t != tag);
        }
        self.prune(event_id);
        on
    }

    pub fn set_note(&mut self, event_id: u32, note: &str) {
        self.entry(event_id).note = note.to_owned();
        self.prune(event_id);
    }

    pub fn reviewed_among(&self, deaths: &[Death]) -> usize {
        deaths
            .iter()
            .filter(|d| self.is_reviewed(d.event_id))
            .count()
    }

    pub fn summary(&self, deaths: &[Death]) -> ReviewSummary {
        let reviews: Vec<&DeathReview> =
            deaths.iter().filter_map(|d| self.get(d.event_id)).collect();
        let mut tags: Vec<(DeathTag, usize)> = DeathTag::ALL
            .iter()
            .map(|tag| {
                let count = reviews.iter().filter(|r| r.tags.contains(tag)).count();
                (*tag, count)
            })
            .filter(|(_, count)| *count > 0)
            .collect();
        tags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.order().cmp(&b.0.order())));
        ReviewSummary {
            deaths: deaths.len(),
            reviewed: reviews.len(),
            notes: reviews.iter().filter(|r| !r.note.trim().is_empty()).count(),
            tags,
        }
    }

    fn entry(&mut self, event_id: u32) -> &mut DeathReview {
        let at = match self.deaths.iter().position(|d| d.event_id == event_id) {
            Some(at) => at,
            None => {
                self.deaths.push(DeathReview {
                    event_id,
                    ..DeathReview::default()
                });
                self.deaths.sort_by_key(|d| d.event_id);
                self.deaths
                    .iter()
                    .position(|d| d.event_id == event_id)
                    .unwrap_or(0)
            }
        };
        &mut self.deaths[at]
    }

    fn prune(&mut self, event_id: u32) {
        self.deaths
            .retain(|d| d.event_id != event_id || !d.is_blank());
    }
}

#[cfg(test)]
mod tests;
