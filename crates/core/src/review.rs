use crate::game::{GameEvent, Player};
use crate::library::EventEntry;

pub const WINDOW_BEFORE_SECS: f64 = 12.0;
pub const WINDOW_AFTER_SECS: f64 = 3.0;
pub const SLOW_MO_TAIL_SECS: f64 = 4.0;

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

#[cfg(test)]
mod tests;
