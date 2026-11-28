mod event;
mod rebind;

use greyscreen_core::keybinds::{Action, Scope};
use greyscreen_core::library::EventEntry;
use greyscreen_core::review::DeathTag;
use rune::{Key, KeyEvent, Tx};

use super::review::jump::{next_target, previous_target};
use super::{Detail, PanelTab, Screen, Source, Ui, VolumeHover};

pub(crate) use event::MAC;
use event::combo_of;

const STEP_SECS: f64 = 5.0;

impl Ui {
    pub(crate) fn on_key(&mut self, ev: &KeyEvent, tx: &mut Tx) -> bool {
        if self.rebinding.is_some() {
            return self.capture_binding(ev);
        }
        if !self.settings_open
            && self.review.as_ref().is_some_and(|r| r.death_active())
            && !ev.mods.ctrl
            && !ev.mods.cmd
            && !ev.mods.alt
            && let Key::Text(ref t) = ev.key
            && t.len() == 1
            && let Some(idx) = t
                .bytes()
                .next()
                .and_then(|b| b.checked_sub(b'1'))
                .filter(|&i| (i as usize) < DeathTag::ALL.len())
        {
            self.death_toggle_tag(DeathTag::ALL[idx as usize], tx);
            return true;
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
            Action::SendFeedback if self.in_theater() => return false,
            Action::SendFeedback => self.open_feedback(tx),
            _ if self.settings_open => return false,
            Action::GoHome => self.go(Screen::Home),
            Action::GoGames => self.go(Screen::Games),
            Action::GoClips => self.go(Screen::Clips),
            Action::FocusSearch => {
                self.go(Screen::Games);
                tx.focus("filter-champion", true);
            }
            Action::PreviousGame | Action::NextGame if self.screen != Screen::Games => {
                return false;
            }
            Action::PreviousGame => self.step_selection(-1),
            Action::NextGame => self.step_selection(1),
            Action::FocusPlayer if self.detail() == Detail::Game => tx.focus("review-video", true),
            Action::FocusPlayer => return false,
            Action::ReviewDeaths if self.detail() == Detail::GamePage => {
                self.review_game_page_deaths()
            }
            Action::ExitFullscreen if self.detail() == Detail::GamePage => self.close_game(),
            Action::ToggleGamesList
                if self.screen == Screen::Games
                    && self.detail() == Detail::Game
                    && !self.in_theater()
                    && self.review.as_ref().is_none_or(|r| r.death.is_none()) =>
            {
                tx.instant();
                self.change(|s| s.games_list_hidden = !s.games_list_hidden);
            }
            Action::ToggleGamesList => return false,
            _ => return self.run_review_action(action, tx),
        }
        true
    }

    fn run_review_action(&mut self, action: Action, tx: &mut Tx) -> bool {
        if self.detail() != Detail::Game {
            return false;
        }
        let theater = self.in_theater();
        let leaving_theater = theater && action == Action::ExitFullscreen;
        if !leaving_theater && self.run_death_action(action) {
            self.wake_after_key(tx);
            return true;
        }
        match action {
            Action::ExitFullscreen if theater => self.set_theater(false, tx),
            Action::ExitFullscreen => self.close_game(),
            Action::ToggleTheater => self.set_theater(!theater, tx),
            Action::ToggleFullscreen => self.set_full_screen(!self.in_full_screen(), tx),
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
            Action::ToggleInspector
                if !theater && self.review.as_ref().is_none_or(|r| r.death.is_none()) =>
            {
                tx.instant();
                self.change(|s| s.review_inspector_hidden = !s.review_inspector_hidden);
            }
            Action::ToggleInspector
            | Action::ToggleGamesList
            | Action::ReviewDeaths
            | Action::NextDeath
            | Action::PreviousDeath
            | Action::ReplayDeath
            | Action::SaveClip
            | Action::OpenSettings
            | Action::GoHome
            | Action::GoGames
            | Action::GoClips
            | Action::PreviousGame
            | Action::NextGame
            | Action::FocusPlayer
            | Action::FocusSearch
            | Action::SendFeedback => return false,
        }
        self.wake_after_key(tx);
        true
    }

    pub(crate) fn video_click(&mut self, tx: &mut Tx) {
        self.run_review_action(Action::PlayPause, tx);
    }

    fn wake_after_key(&mut self, tx: &mut Tx) {
        if let Some(review) = &mut self.review {
            review.wake_controls(tx.clock());
        }
        self.sync_full_screen(tx);
    }

    pub(crate) fn in_theater(&self) -> bool {
        self.review.as_ref().is_some_and(|r| r.theater)
    }

    pub(crate) fn in_full_screen(&self) -> bool {
        self.review.as_ref().is_some_and(|r| r.full_screen)
    }

    pub(crate) fn set_theater(&mut self, on: bool, tx: &mut Tx) {
        if let Some(review) = &mut self.review {
            review.theater = on;
            review.drawer &= on;
            if !on {
                review.full_screen = false;
            }
            review.wake_controls(tx.clock());
        }
        self.sync_full_screen(tx);
    }

    pub(crate) fn set_full_screen(&mut self, on: bool, tx: &mut Tx) {
        if let Some(review) = &mut self.review {
            review.full_screen = on;
            review.theater = on;
            review.drawer &= on;
            review.wake_controls(tx.clock());
        }
        self.sync_full_screen(tx);
    }

    pub(crate) fn sync_full_screen(&mut self, tx: &mut Tx) {
        let want = self.in_full_screen();
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
            self.sound = review.sound();
        }
    }

    pub(crate) fn review_set_volume(&mut self, volume: f32) {
        if let Some(review) = &mut self.review {
            review.set_volume(volume);
            self.sound = review.sound();
        }
    }

    pub(crate) fn review_step_volume(&mut self, delta: i32) {
        if let Some(review) = &self.review {
            let steps = (review.shown_volume() * 20.0).round() + delta as f32;
            self.review_set_volume(steps / 20.0);
        }
    }

    pub(crate) fn review_volume_hover(&mut self, update: impl FnOnce(&mut VolumeHover)) {
        if let Some(review) = &mut self.review {
            update(&mut review.volume_hover);
        }
    }

    pub(crate) fn review_speed_menu(&mut self, open: bool) {
        if let Some(review) = &mut self.review {
            review.speed_menu = open;
        }
    }

    pub(crate) fn review_health(&mut self, open: bool) {
        if let Some(review) = &mut self.review {
            review.health_open = open;
            if open {
                review.more_menu = false;
            }
        }
    }

    pub(crate) fn review_more_menu(&mut self, open: bool) {
        if let Some(review) = &mut self.review {
            review.more_menu = open;
            if open {
                review.health_open = false;
            }
        }
    }

    fn review_step_speed(&mut self, delta: i32) {
        if let Some(review) = &mut self.review {
            review.step_speed(delta);
        }
    }

    pub(crate) fn review_set_speed(&mut self, index: usize) {
        if let Some(review) = &mut self.review {
            review.set_speed(index);
        }
    }

    pub(crate) fn review_tab(&mut self, tab: PanelTab) {
        if let Some(review) = &mut self.review {
            review.tab = tab;
            review.filter_menu = false;
        }
    }

    pub(crate) fn review_drawer(&mut self, open: bool) {
        if let Some(review) = &mut self.review {
            review.drawer = open;
        }
    }

    fn review_toggle(&mut self) {
        if let Some(review) = &mut self.review {
            review.toggle();
        }
    }

    fn review_jump(&mut self, pick: fn(&[EventEntry], f64) -> Option<f64>) {
        let Some(review) = &mut self.review else {
            return;
        };
        if review.source != Source::Game {
            return;
        }
        let shown: Vec<EventEntry> = review.shown_events().cloned().collect();
        if let Some(target) = pick(&shown, review.time()) {
            review.seek(target);
        }
    }
}
