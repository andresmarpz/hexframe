use greyscreen_core::keybinds::{Action, Scope};
use greyscreen_core::library::EventEntry;
use rune::{KeyEvent, Tx};

use super::jump::{next_target, previous_target};
use super::key_event::combo_of;
use super::{EventFilter, PanelTab, Screen, Source, Ui};

const STEP_SECS: f64 = 5.0;

impl Ui {
    pub(crate) fn on_key(&mut self, ev: &KeyEvent, tx: &mut Tx) -> bool {
        if self.rebinding.is_some() {
            return self.capture_binding(ev);
        }
        let Some(combo) = combo_of(ev) else {
            return false;
        };
        match self.settings.keymap().action_for(Scope::App, &combo) {
            Some(action) => self.run_action(action, tx),
            None => false,
        }
    }

    fn run_action(&mut self, action: Action, tx: &mut Tx) -> bool {
        match action {
            Action::OpenSettings => self.toggle_settings(),
            Action::GoHome => self.go(Screen::Home),
            Action::GoRecordings => self.go(Screen::Recordings),
            _ if self.settings_open => return false,
            _ => return self.run_review_action(action, tx),
        }
        true
    }

    fn run_review_action(&mut self, action: Action, tx: &mut Tx) -> bool {
        if self.screen != Screen::Review || self.review.is_none() {
            return false;
        }
        let theater = self.in_theater();
        match action {
            Action::ExitFullscreen if theater => self.set_theater(false, tx),
            Action::ExitFullscreen => self.go(Screen::Recordings),
            Action::ToggleFullscreen => self.set_theater(!theater, tx),
            Action::PlayPause => self.review_toggle(),
            Action::SeekBack => self.review_step(-STEP_SECS),
            Action::SeekForward => self.review_step(STEP_SECS),
            Action::PreviousEvent => self.review_jump(previous_target),
            Action::NextEvent => self.review_jump(next_target),
            Action::FrameBack => self.review_frame(-1.0),
            Action::FrameForward => self.review_frame(1.0),
            Action::Mute => self.review_mute(),
            Action::SpeedDown => self.review_step_speed(-1),
            Action::SpeedUp => self.review_step_speed(1),
            Action::SaveClip | Action::OpenSettings | Action::GoHome | Action::GoRecordings => {
                return false;
            }
        }
        if let Some(review) = &mut self.review {
            review.wake_controls(tx.clock());
        }
        self.sync_full_screen(tx);
        true
    }

    pub(crate) fn in_theater(&self) -> bool {
        self.review.as_ref().is_some_and(|r| r.theater)
    }

    pub(crate) fn set_theater(&mut self, on: bool, tx: &mut Tx) {
        if let Some(review) = &mut self.review {
            review.theater = on;
            review.drawer &= on;
            review.wake_controls(tx.clock());
        }
        self.sync_full_screen(tx);
    }

    pub(crate) fn sync_full_screen(&mut self, tx: &mut Tx) {
        let want = self.in_theater();
        if want != self.window_full_screen {
            self.window_full_screen = want;
            tx.toggle_full_screen();
        }
    }

    pub(crate) fn review_step(&mut self, delta: f64) {
        if let Some(review) = &mut self.review {
            review.seek(review.time() + delta);
        }
    }

    fn review_frame(&mut self, direction: f64) {
        if let Some(review) = &mut self.review {
            review.step_frame(direction);
        }
    }

    pub(crate) fn review_mute(&mut self) {
        if let Some(review) = &mut self.review {
            review.toggle_mute();
        }
    }

    fn review_step_speed(&mut self, delta: i32) {
        if let Some(review) = &mut self.review {
            review.step_speed(delta);
        }
    }

    pub(crate) fn review_cycle_speed(&mut self) {
        if let Some(review) = &mut self.review {
            review.cycle_speed();
        }
    }

    pub(crate) fn review_tab(&mut self, tab: PanelTab) {
        if let Some(review) = &mut self.review {
            review.tab = tab;
        }
    }

    pub(crate) fn review_filter(&mut self, filter: EventFilter) {
        if let Some(review) = &mut self.review {
            review.filter = filter;
        }
    }

    pub(crate) fn review_drawer(&mut self, open: bool) {
        if let Some(review) = &mut self.review {
            review.drawer = open;
        }
    }

    fn review_toggle(&self) {
        if let Some(review) = &self.review {
            review.toggle();
        }
    }

    fn review_jump(&mut self, pick: fn(&[EventEntry], f64) -> Option<f64>) {
        if let Some(review) = &mut self.review
            && review.source == Source::Game
            && let Some(target) = pick(&review.entry.meta.events, review.time())
        {
            review.seek(target);
        }
    }
}
