mod fixture;
mod scenes;

use greyscreen_core::library::{META_FILE, write_review};
use greyscreen_core::review::{Death, DeathTag, SLOW_MO_RATE, deaths, killer_label};
use greyscreen_engine::NoticeLevel;
use rune::{TextEdit, Tx};

use super::format_clock;
use super::{PlayerFactory, ReviewState, Source, Ui};

#[cfg(test)]
pub(crate) use fixture::death_entry;
pub(crate) use scenes::{scene_demo, scene_mid, scene_summary};

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
        if let Some(player) = &self.player {
            player.pause();
        }
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
        if playing && time >= end {
            player.pause();
            state.finished = true;
            changed = true;
        }
        changed
    }
}

impl Ui {
    pub(crate) fn review_deaths(&mut self) {
        let make = self.player_factory;
        let began = self
            .review
            .as_mut()
            .is_some_and(|review| review.begin_death_review(make));
        if began {
            self.sync_death_note();
        }
    }

    pub(crate) fn death_go(&mut self, index: usize) {
        if let Some(review) = &mut self.review {
            review.go_to_death(index);
        }
        self.sync_death_note();
    }

    pub(crate) fn death_next(&mut self) {
        let next = self
            .review
            .as_ref()
            .and_then(|r| r.death.as_ref())
            .filter(|d| d.has_next())
            .map(|d| d.index + 1);
        if let Some(next) = next {
            self.death_go(next);
        }
    }

    pub(crate) fn death_previous(&mut self) {
        let previous = self
            .review
            .as_ref()
            .and_then(|r| r.death.as_ref())
            .filter(|d| d.has_previous())
            .map(|d| d.index - 1);
        if let Some(previous) = previous {
            self.death_go(previous);
        }
    }

    pub(crate) fn death_replay(&mut self) {
        if let Some(review) = &mut self.review {
            review.replay_death();
        }
    }

    pub(crate) fn death_scrub(&mut self, fraction: f64) {
        if let Some(review) = &mut self.review {
            review.scrub_death(fraction);
        }
    }

    pub(crate) fn death_slow_mo(&mut self) {
        if let Some(review) = &mut self.review {
            review.toggle_slow_mo();
        }
    }

    pub(crate) fn death_finish(&mut self) {
        if let Some(review) = &mut self.review {
            review.finish_death_review();
        }
    }

    pub(crate) fn death_close(&mut self) {
        if let Some(review) = &mut self.review {
            review.close_death_review();
        }
    }

    pub(crate) fn death_again(&mut self) {
        self.death_go(0);
    }

    pub(crate) fn death_toggle_tag(&mut self, tag: DeathTag, tx: &mut Tx) {
        let Some(review) = &mut self.review else {
            return;
        };
        let Some(id) = review
            .death
            .as_ref()
            .and_then(|d| d.current())
            .map(|d| d.event_id)
        else {
            return;
        };
        review.entry.meta.review.toggle_tag(id, tag);
        self.save_review(tx);
    }

    pub(crate) fn death_note_edited(&mut self, tx: &mut Tx) {
        let Some(review) = &mut self.review else {
            return;
        };
        let Some(id) = review
            .death
            .as_ref()
            .and_then(|d| d.current())
            .map(|d| d.event_id)
        else {
            return;
        };
        review.entry.meta.review.set_note(id, &self.death_note.text);
        self.save_review(tx);
    }

    fn sync_death_note(&mut self) {
        let note = self
            .review
            .as_ref()
            .and_then(|r| r.death.as_ref().and_then(|d| d.current()).map(|d| (r, d)))
            .map(|(r, d)| r.entry.meta.review.note(d.event_id).to_owned())
            .unwrap_or_default();
        self.death_note = TextEdit::new(note);
    }

    fn save_review(&mut self, tx: &mut Tx) {
        let Some(review) = &self.review else { return };
        let data = review.entry.meta.review.clone();
        let id = review.entry.meta.id.clone();
        let folder = review.entry.folder.clone();
        for rec in self.recordings.iter_mut().chain(self.last_game.iter_mut()) {
            if rec.meta.id == id {
                rec.meta.review = data.clone();
            }
        }
        if folder.join(META_FILE).is_file()
            && let Err(err) = write_review(&folder, &data)
        {
            tracing::warn!(%err, "cannot save review notes");
            self.show_notice(
                NoticeLevel::Error,
                "Could not save your review notes".into(),
                tx,
            );
        }
    }

    pub(crate) fn run_death_action(&mut self, action: greyscreen_core::keybinds::Action) -> bool {
        use greyscreen_core::keybinds::Action;
        let Some(review) = &self.review else {
            return false;
        };
        if review.death_summary_open() {
            match action {
                Action::ExitFullscreen => self.death_close(),
                Action::ReviewDeaths => self.death_again(),
                _ => {}
            }
            return true;
        }
        if !review.death_active() {
            if action == Action::ReviewDeaths {
                self.review_deaths();
                return true;
            }
            return false;
        }
        let finished = review.death.as_ref().is_some_and(|d| d.finished);
        match action {
            Action::ExitFullscreen => self.death_finish(),
            Action::NextDeath | Action::SeekForward | Action::NextEvent => self.death_next(),
            Action::PreviousDeath | Action::SeekBack | Action::PreviousEvent => {
                self.death_previous();
            }
            Action::ReplayDeath => self.death_replay(),
            Action::PlayPause if finished => self.death_replay(),
            Action::ReviewDeaths | Action::ToggleFullscreen => {}
            _ => return false,
        }
        true
    }
}
