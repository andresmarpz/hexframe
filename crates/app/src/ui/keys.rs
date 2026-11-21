use greyscreen_core::library::EventEntry;
use rune::{Key, KeyEvent, Tx};

use super::jump::{next_target, previous_target};
use super::{EventFilter, PanelTab, Screen, Source, Ui};

const STEP_SECS: f64 = 5.0;

impl Ui {
    pub(crate) fn on_key(&mut self, ev: &KeyEvent, tx: &mut Tx) -> bool {
        if self.hotkey_capture {
            return self.capture_hotkey(ev);
        }
        let m = ev.mods;
        if self.screen != Screen::Review
            || self.review.is_none()
            || m.cmd
            || m.ctrl
            || m.alt
            || m.logo
        {
            return false;
        }
        let theater = self.in_theater();
        match &ev.key {
            Key::Escape if theater => self.set_theater(false, tx),
            Key::Escape => self.go(Screen::Recordings),
            Key::Left => self.review_step(-STEP_SECS),
            Key::Right => self.review_step(STEP_SECS),
            Key::Text(t) => match t.to_lowercase().as_str() {
                " " => self.review_toggle(),
                "f" => self.set_theater(!theater, tx),
                "j" => self.review_jump(previous_target),
                "l" => self.review_jump(next_target),
                "," => self.review_frame(-1.0),
                "." => self.review_frame(1.0),
                "m" => self.review_mute(),
                _ => return false,
            },
            _ => return false,
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
