use greyscreen_core::keybinds::Action;
use greyscreen_engine::RecordingEntry;

use crate::ui::MAC;
use crate::ui::Ui;

pub(crate) const RECENT_LIMIT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct LibraryTotals {
    pub bytes: u64,
}

impl LibraryTotals {
    pub(crate) fn of(recordings: &[RecordingEntry]) -> LibraryTotals {
        LibraryTotals {
            bytes: recordings.iter().map(|rec| rec.size_bytes).sum(),
        }
    }

    pub(crate) fn used_fraction(&self, limit_bytes: Option<u64>) -> Option<f32> {
        let limit = limit_bytes.filter(|limit| *limit > 0)?;
        Some((self.bytes as f64 / limit as f64).clamp(0.0, 1.0) as f32)
    }
}

impl Ui {
    pub(crate) fn toggle_nav(&mut self) {
        self.change(|s| s.sidebar_collapsed = !s.sidebar_collapsed);
    }

    pub(crate) fn nav_shortcut(&self) -> String {
        self.settings
            .keymap()
            .bindings(Action::ToggleNav)
            .first()
            .map_or_else(|| "unbound".to_owned(), |combo| combo.display(MAC))
    }
}
