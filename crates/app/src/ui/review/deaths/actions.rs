use greyscreen_core::keybinds::Action;
use greyscreen_core::library::{META_FILE, write_review};
use greyscreen_core::review::DeathTag;
use greyscreen_engine::NoticeLevel;
use rune::{TextEdit, Tx};

use crate::ui::Ui;

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
        let Some(id) = review.current_death_event() else {
            return;
        };
        review.entry.meta.review.toggle_tag(id, tag);
        self.save_review(tx);
    }

    pub(crate) fn death_note_edited(&mut self, tx: &mut Tx) {
        let Some(review) = &mut self.review else {
            return;
        };
        let Some(id) = review.current_death_event() else {
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

    pub(crate) fn run_death_action(&mut self, action: Action) -> bool {
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
