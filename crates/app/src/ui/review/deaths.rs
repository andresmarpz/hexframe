mod actions;

use greyscreen_core::review::{Death, SLOW_MO_RATE, deaths, killer_label};

use super::{PlayerFactory, ReviewState, Source};
use crate::ui::format_clock;

const ARM_SLACK_SECS: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeathReviewState {
    pub deaths: Vec<Death>,
    pub index: usize,
    pub slow_mo: bool,
    pub slowed: bool,
    pub armed: bool,
    pub finished: bool,
    pub summary: bool,
}

impl DeathReviewState {
    fn new(deaths: Vec<Death>) -> Self {
        DeathReviewState {
            deaths,
            index: 0,
            slow_mo: false,
            slowed: false,
            armed: false,
            finished: false,
            summary: false,
        }
    }

    pub(crate) fn current(&self) -> Option<&Death> {
        self.deaths.get(self.index)
    }

    pub(crate) fn has_next(&self) -> bool {
        self.index + 1 < self.deaths.len()
    }

    pub(crate) fn has_previous(&self) -> bool {
        self.index > 0
    }
}

fn killer_line(death: &Death, review: &ReviewState) -> String {
    let players = &review.entry.meta.game.info.players;
    let killer = killer_label(&death.killer, players);
    let mut line = format!("Killed by {killer}");
    let helpers: Vec<String> = death
        .assisters
        .iter()
        .map(|name| killer_label(name, players))
        .collect();
    match helpers.as_slice() {
        [] => {}
        [one] => line.push_str(&format!(" (with {one})")),
        [rest @ .., last] => line.push_str(&format!(" (with {} and {last})", rest.join(", "))),
    }
    line
}

pub(crate) fn title_line(state: &DeathReviewState, review: &ReviewState) -> String {
    let Some(death) = state.current() else {
        return String::new();
    };
    format!(
        "Death {} of {} · {} · {}",
        state.index + 1,
        state.deaths.len(),
        format_clock(death.game_secs.max(0.0) as u32),
        killer_line(death, review)
    )
}

impl ReviewState {
    pub(crate) fn death_list(&self) -> Vec<Death> {
        let meta = &self.entry.meta;
        deaths(&meta.events, &meta.game.info.me)
    }

    pub(crate) fn death_count(&self) -> usize {
        self.death_list().len()
    }

    pub(crate) fn death_active(&self) -> bool {
        self.death.as_ref().is_some_and(|d| !d.summary)
    }

    pub(crate) fn death_summary_open(&self) -> bool {
        self.death.as_ref().is_some_and(|d| d.summary)
    }

    pub(crate) fn death_finished(&self) -> bool {
        self.death
            .as_ref()
            .is_some_and(|d| d.finished && !d.summary)
    }

    pub(crate) fn death_window(&self) -> Option<(f64, f64)> {
        let death = self.death.as_ref()?.current()?;
        Some(death.window(self.duration()))
    }

    pub(crate) fn begin_death_review(&mut self, make: PlayerFactory) -> bool {
        let list = self.death_list();
        if list.is_empty() {
            return false;
        }
        if self.source != Source::Game {
            self.play_game(make);
        }
        if self.player.is_none() {
            return false;
        }
        self.death = Some(DeathReviewState::new(list));
        self.go_to_death(0);
        true
    }

    pub(crate) fn go_to_death(&mut self, index: usize) {
        let Some(state) = &mut self.death else { return };
        if index >= state.deaths.len() {
            return;
        }
        state.index = index;
        state.summary = false;
        self.replay_death();
    }

    pub(crate) fn replay_death(&mut self) {
        let Some((start, _)) = self.death_window() else {
            return;
        };
        if let Some(state) = &mut self.death {
            state.finished = false;
            state.armed = false;
            state.slowed = false;
            state.summary = false;
        }
        self.apply_death_rate(false);
        self.seek(start);
        if let Some(player) = &self.player {
            player.play();
        }
    }

    fn apply_death_rate(&self, slowed: bool) {
        if let Some(player) = &self.player {
            let rate = if slowed {
                SLOW_MO_RATE
            } else {
                self.speed_rate()
            };
            player.set_rate(rate);
        }
    }

    pub(crate) fn scrub_death(&mut self, fraction: f64) {
        let Some((start, end)) = self.death_window() else {
            return;
        };
        if let Some(state) = &mut self.death {
            state.finished = false;
            state.armed = false;
            state.slowed = false;
        }
        self.apply_death_rate(false);
        self.seek(start + (end - start) * fraction.clamp(0.0, 1.0));
    }

    pub(crate) fn toggle_slow_mo(&mut self) {
        if let Some(state) = &mut self.death {
            state.slow_mo = !state.slow_mo;
        }
        self.death_tick();
    }

    pub(crate) fn finish_death_review(&mut self) {
        if let Some(state) = &mut self.death {
            state.summary = true;
        }
        self.pause();
        self.apply_death_rate(false);
    }

    pub(crate) fn close_death_review(&mut self) {
        self.death = None;
        self.apply_death_rate(false);
    }

    pub(crate) fn death_tick(&mut self) -> bool {
        let Some(player) = &self.player else {
            return false;
        };
        let Some((start, end)) = self.death_window() else {
            return false;
        };
        let time = player.time();
        let playing = self.is_playing();
        let duration = self.duration();
        let normal_rate = self.speed_rate();
        let Some(state) = &mut self.death else {
            return false;
        };
        if state.summary {
            return false;
        }
        if !state.armed {
            if time < start - ARM_SLACK_SECS || time > end {
                return false;
            }
            state.armed = true;
        }
        let mut changed = false;
        let slow_from = state.current().map_or(end, |d| d.slow_mo_from(duration));
        let wants_slow = state.slow_mo && time >= slow_from && time < end;
        if wants_slow != state.slowed {
            state.slowed = wants_slow;
            changed = true;
            player.set_rate(if wants_slow {
                SLOW_MO_RATE
            } else {
                normal_rate
            });
        }
        let finish = playing && time >= end && !state.finished;
        if finish {
            state.finished = true;
            self.pause();
        }
        changed || finish
    }
}
