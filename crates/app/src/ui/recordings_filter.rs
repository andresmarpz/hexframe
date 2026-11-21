use greyscreen_core::game::GameResult;
use greyscreen_engine::{EngineCmd, RecordingEntry};
use rune::TextEdit;

use super::Ui;
use super::library_format::mode_label;

pub(crate) const BIN: &str = if cfg!(windows) {
    "Recycle Bin"
} else {
    "Trash"
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ResultFilter {
    #[default]
    All,
    Victory,
    Defeat,
}

impl ResultFilter {
    pub(crate) const ALL: [ResultFilter; 3] = [Self::All, Self::Victory, Self::Defeat];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Victory => "Victory",
            Self::Defeat => "Defeat",
        }
    }

    fn accepts(self, result: GameResult) -> bool {
        match self {
            Self::All => true,
            Self::Victory => result == GameResult::Win,
            Self::Defeat => result == GameResult::Loss,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct RecordingsFilter {
    pub champion: TextEdit,
    pub result: ResultFilter,
    pub mode: Option<String>,
    pub mode_open: bool,
    pub pending_delete: Option<String>,
}

impl RecordingsFilter {
    pub(crate) fn is_active(&self) -> bool {
        !self.champion.text.trim().is_empty()
            || self.result != ResultFilter::All
            || self.mode.is_some()
    }

    pub(crate) fn clear(&mut self) {
        self.champion.clear();
        self.result = ResultFilter::All;
        self.mode = None;
        self.mode_open = false;
    }

    pub(crate) fn accepts(&self, rec: &RecordingEntry) -> bool {
        let needle = self.champion.text.trim().to_lowercase();
        self.result.accepts(rec.meta.game.result)
            && self
                .mode
                .as_deref()
                .is_none_or(|mode| mode_label(&rec.meta.game.info.mode) == mode)
            && (needle.is_empty()
                || rec
                    .meta
                    .game
                    .info
                    .me
                    .champion
                    .to_lowercase()
                    .contains(&needle))
    }

    pub(crate) fn apply<'a>(&self, recordings: &'a [RecordingEntry]) -> Vec<&'a RecordingEntry> {
        recordings.iter().filter(|rec| self.accepts(rec)).collect()
    }
}

pub(crate) fn modes_present(recordings: &[RecordingEntry]) -> Vec<String> {
    let mut modes: Vec<String> = recordings
        .iter()
        .map(|rec| mode_label(&rec.meta.game.info.mode))
        .filter(|label| !label.is_empty())
        .collect();
    modes.sort();
    modes.dedup();
    modes
}

impl Ui {
    pub(crate) fn ask_delete(&mut self, id: String) {
        self.filter.pending_delete = Some(id);
    }

    pub(crate) fn cancel_delete(&mut self) {
        self.filter.pending_delete = None;
    }

    pub(crate) fn confirm_delete(&mut self) {
        let Some(id) = self.filter.pending_delete.take() else {
            return;
        };
        if self.last_game.as_ref().is_some_and(|rec| rec.meta.id == id) {
            self.last_game = None;
        }
        self.send(EngineCmd::DeleteRecording { id });
    }

    pub(crate) fn pending_delete_entry(&self) -> Option<&RecordingEntry> {
        let id = self.filter.pending_delete.as_deref()?;
        self.recordings.iter().find(|rec| rec.meta.id == id)
    }
}

#[cfg(test)]
mod tests;
