use greyscreen_core::library::EventEntry;

use super::{ReviewState, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EventFilter {
    #[default]
    All,
    Deaths,
    Kills,
    Objectives,
}

impl EventFilter {
    pub(crate) const EVERY: [EventFilter; 4] = [
        EventFilter::All,
        EventFilter::Deaths,
        EventFilter::Kills,
        EventFilter::Objectives,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            EventFilter::All => "All",
            EventFilter::Deaths => "Deaths",
            EventFilter::Kills => "Kills",
            EventFilter::Objectives => "Objectives",
        }
    }

    pub(crate) fn matches(self, event: &EventEntry) -> bool {
        let any = |wanted: fn(&str) -> bool| event.highlights.iter().any(|h| wanted(h));
        match self {
            EventFilter::All => true,
            EventFilter::Deaths => any(|h| h == "MyDeath"),
            EventFilter::Kills => any(is_kill_label),
            EventFilter::Objectives => any(is_objective_label),
        }
    }
}

fn is_kill_label(label: &str) -> bool {
    matches!(label, "MyKill" | "MyAssist" | "FirstBlood" | "Ace") || label.starts_with("Multikill")
}

fn is_objective_label(label: &str) -> bool {
    let label = label.strip_prefix("Enemy").unwrap_or(label);
    matches!(
        label,
        "Baron" | "Dragon" | "Herald" | "VoidGrub" | "Turret" | "Inhibitor"
    )
}

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
            .filter(move |e| filter.matches(e))
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
