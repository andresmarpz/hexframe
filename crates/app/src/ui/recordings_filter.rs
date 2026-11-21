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

#[derive(Debug, Default)]
pub(crate) struct RecordingsFilter {
    pub champion: TextEdit,
    pub pending_delete: Option<String>,
}

impl RecordingsFilter {
    pub(crate) fn is_active(&self) -> bool {
        !self.champion.text.trim().is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.champion.clear();
    }

    pub(crate) fn accepts(&self, rec: &RecordingEntry) -> bool {
        let needle = self.champion.text.trim().to_lowercase();
        let game = &rec.meta.game;
        let result = match game.result {
            GameResult::Win => "victory",
            GameResult::Loss => "defeat",
            GameResult::Unknown => "",
        };
        needle.is_empty()
            || [
                game.info.me.champion.to_lowercase(),
                mode_label(&game.info.mode).to_lowercase(),
                result.to_owned(),
            ]
            .iter()
            .any(|field| field.contains(&needle))
    }

    pub(crate) fn apply<'a>(&self, recordings: &'a [RecordingEntry]) -> Vec<&'a RecordingEntry> {
        recordings.iter().filter(|rec| self.accepts(rec)).collect()
    }
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
