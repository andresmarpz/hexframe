use greyscreen_engine::{EngineStatus, RecordingEntry};

use super::format::{day_heading, death_total};
use crate::ui::{Screen, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Detail {
    Welcome,
    Pick,
    Clips,
    Live,
    Game,
    GamePage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ListItem {
    Live,
    Day(String),
    Game(usize),
}

impl Ui {
    pub(crate) fn live_row_visible(&self) -> bool {
        matches!(
            self.status,
            EngineStatus::Recording { .. } | EngineStatus::Finalizing
        )
    }

    pub(crate) fn detail(&self) -> Detail {
        match self.screen {
            Screen::Home if self.live_row_visible() => Detail::Live,
            Screen::Home => Detail::Welcome,
            Screen::Games if self.live_selected && self.live_row_visible() => Detail::Live,
            Screen::Games if self.review.is_some() => Detail::Game,
            Screen::Games if self.game_page.is_some() => Detail::GamePage,
            Screen::Games => Detail::Pick,
            Screen::Clips => Detail::Clips,
        }
    }

    pub(crate) fn selected_id(&self) -> Option<&str> {
        self.review.as_ref().map(|r| r.entry.meta.id.as_str())
    }

    pub(crate) fn select_live(&mut self) {
        if self.live_row_visible() {
            self.live_selected = true;
            self.review = None;
            self.screen = Screen::Games;
        }
    }

    pub(crate) fn close_game(&mut self) {
        self.live_selected = false;
        if self.review.is_none() || self.game_page.is_none() {
            self.game_page = None;
        }
        self.review = None;
    }

    pub(crate) fn open_game(&mut self, id: String) {
        let Some(entry) = self.recordings.iter().find(|r| r.meta.id == id) else {
            return;
        };
        let id = entry.meta.id.clone();
        self.close_settings();
        if let Some(review) = &mut self.review {
            review.pause();
        }
        self.review = None;
        self.live_selected = false;
        self.game_page = Some(id);
        self.screen = Screen::Games;
    }

    pub(crate) fn game_page_entry(&self) -> Option<&RecordingEntry> {
        let id = self.game_page.as_deref()?;
        self.recordings.iter().find(|r| r.meta.id == id)
    }

    fn open_page_review(&mut self) -> bool {
        let Some(entry) = self.game_page_entry().cloned() else {
            return false;
        };
        let id = entry.meta.id.clone();
        self.open_review_entry(entry);
        self.game_page = Some(id);
        true
    }

    pub(crate) fn reveal_game_page_folder(&self) {
        if let Some(entry) = self.game_page_entry() {
            crate::headless::reveal(&entry.folder);
        }
    }

    pub(crate) fn play_game_page(&mut self) {
        if self.open_page_review() {
            self.play_game();
        }
    }

    pub(crate) fn review_game_page_deaths(&mut self) {
        let has_deaths = self.game_page_entry().is_some_and(|e| death_total(e) > 0);
        if has_deaths && self.open_page_review() {
            self.review_deaths();
        }
    }

    pub(crate) fn apply_library(&mut self, recordings: Vec<RecordingEntry>) {
        self.recordings = recordings;
        let gone = self
            .selected_id()
            .is_some_and(|id| !self.recordings.iter().any(|rec| rec.meta.id == id));
        if gone {
            self.review = None;
        }
        if self.game_page_entry().is_none() {
            self.game_page = None;
        }
    }

    pub(crate) fn visible_games(&self) -> Vec<usize> {
        let mut games: Vec<usize> = self
            .recordings
            .iter()
            .enumerate()
            .filter(|(_, rec)| self.filter.accepts(rec))
            .map(|(i, _)| i)
            .collect();
        games.sort_by_key(|i| std::cmp::Reverse(self.recordings[*i].meta.started_at.timestamp()));
        games
    }

    pub(crate) fn list_items(&self) -> Vec<ListItem> {
        let now = jiff::Zoned::now();
        let mut items = Vec::new();
        if self.live_row_visible() {
            items.push(ListItem::Live);
        }
        let mut day = String::new();
        for index in self.visible_games() {
            let heading = day_heading(&self.recordings[index].meta.started_at, &now);
            if heading != day {
                items.push(ListItem::Day(heading.clone()));
                day = heading;
            }
            items.push(ListItem::Game(index));
        }
        items
    }

    pub(crate) fn step_selection(&mut self, delta: isize) {
        let mut slots: Vec<Option<usize>> = Vec::new();
        if self.live_row_visible() {
            slots.push(None);
        }
        slots.extend(self.visible_games().into_iter().map(Some));
        let Some(last) = slots.len().checked_sub(1) else {
            return;
        };
        let current = match self.detail() {
            Detail::GamePage => slots.iter().position(|slot| {
                slot.is_some_and(|i| {
                    Some(self.recordings[i].meta.id.as_str()) == self.game_page.as_deref()
                })
            }),
            Detail::Live => slots.iter().position(Option::is_none),
            Detail::Game => slots.iter().position(|slot| {
                slot.is_some_and(|i| {
                    Some(self.recordings[i].meta.id.as_str()) == self.selected_id()
                })
            }),
            Detail::Welcome | Detail::Pick | Detail::Clips => None,
        };
        let next = match current {
            Some(at) => at.saturating_add_signed(delta).min(last),
            None if delta < 0 => last,
            None => 0,
        };
        if Some(next) == current {
            return;
        }
        match slots[next] {
            None => self.select_live(),
            Some(index) => {
                let id = self.recordings[index].meta.id.clone();
                if self.detail() == Detail::GamePage {
                    self.open_game(id);
                } else {
                    let entry = self.recordings[index].clone();
                    self.open_review_entry(entry);
                }
            }
        }
    }
}
