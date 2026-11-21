use greyscreen_engine::{EngineStatus, RecordingEntry};
use jiff::Zoned;

use super::format::day_heading;
use crate::ui::{Screen, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Detail {
    Welcome,
    Pick,
    Clips,
    Live,
    Game,
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
        self.review = None;
    }

    pub(crate) fn apply_library(&mut self, recordings: Vec<RecordingEntry>) {
        self.recordings = recordings;
        let gone = self
            .selected_id()
            .is_some_and(|id| !self.recordings.iter().any(|rec| rec.meta.id == id));
        if gone {
            self.review = None;
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
        let now = Zoned::now();
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
                let entry = self.recordings[index].clone();
                self.open_review_entry(entry);
            }
        }
    }
}
