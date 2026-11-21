use greyscreen_core::categories::{CategorySet, EventCategory};
use greyscreen_core::library::EventEntry;
use rune::Tx;

use super::{ReviewState, Source};
use crate::ui::Ui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventHover {
    pub event_id: u32,
    pub anchor: String,
    pub theater: bool,
}

impl ReviewState {
    pub(crate) fn shown_events(&self) -> impl Iterator<Item = &EventEntry> {
        let filter = self.filter;
        self.entry
            .meta
            .events
            .iter()
            .zip(&self.categories)
            .filter(move |(_, category)| filter.contains(**category))
            .map(|(event, _)| event)
    }

    pub(crate) fn hidden_events(&self) -> usize {
        self.entry.meta.events.len() - self.shown_events().count()
    }

    pub(crate) fn filter_differs_from_default(&self) -> bool {
        self.filter != self.default_filter
    }

    pub(crate) fn set_filter(&mut self, filter: CategorySet) {
        self.filter = filter;
        self.entry.meta.review.filter = (filter != self.default_filter).then_some(filter);
    }

    pub(crate) fn set_default_filter(&mut self, default: CategorySet) {
        self.default_filter = default;
        if self.entry.meta.review.filter == Some(default) {
            self.entry.meta.review.filter = None;
        }
        self.filter = self.entry.meta.review.filter.unwrap_or(default);
    }

    pub(crate) fn active_event(&self) -> Option<u32> {
        if self.source != Source::Game {
            return None;
        }
        let now = self.time();
        self.shown_events()
            .filter(|e| e.rec_secs <= now)
            .max_by(|a, b| a.rec_secs.total_cmp(&b.rec_secs))
            .map(|e| e.id)
    }

    pub(crate) fn hover_event(&mut self, event_id: u32, anchor: &str, inside: bool) {
        if inside {
            self.hovered = Some(EventHover {
                event_id,
                anchor: anchor.to_owned(),
                theater: self.theater,
            });
        } else if self
            .hovered
            .as_ref()
            .is_some_and(|h| h.event_id == event_id && h.anchor == anchor)
        {
            self.hovered = None;
        }
    }

    pub(crate) fn hovered_event(&self) -> Option<(&EventEntry, &str)> {
        let hover = self
            .hovered
            .as_ref()
            .filter(|h| h.theater == self.theater)?;
        let event = self
            .entry
            .meta
            .events
            .iter()
            .find(|e| e.id == hover.event_id)?;
        Some((event, hover.anchor.as_str()))
    }
}

impl Ui {
    pub(crate) fn review_filter_menu(&mut self, open: bool) {
        if let Some(review) = &mut self.review {
            review.filter_menu = open;
        }
    }

    pub(crate) fn review_toggle_category(&mut self, category: EventCategory, tx: &mut Tx) {
        if let Some(review) = &mut self.review {
            review.set_filter(review.filter.toggled(category));
        }
        self.save_review(tx);
    }

    pub(crate) fn review_use_default_filter(&mut self, tx: &mut Tx) {
        if let Some(review) = &mut self.review {
            review.set_filter(review.default_filter);
            review.filter_menu = false;
        }
        self.save_review(tx);
    }

    pub(crate) fn review_save_filter_as_default(&mut self, tx: &mut Tx) {
        let Some(review) = &mut self.review else {
            return;
        };
        let filter = review.filter;
        review.filter_menu = false;
        self.change(|s| s.review_events = filter);
        self.save_review(tx);
    }

    pub(crate) fn sync_review_default(&mut self) {
        let default = self.settings.review_events;
        if let Some(review) = &mut self.review {
            review.set_default_filter(default);
        }
    }
}
