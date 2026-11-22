use rune::{Key, KeyEvent, TextEdit};

use super::{LibraryTotals, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SettingsSection {
    #[default]
    General,
    Recording,
    Clips,
    Storage,
    Keybinds,
}

impl SettingsSection {
    pub(crate) const ALL: [SettingsSection; 5] = [
        Self::General,
        Self::Recording,
        Self::Clips,
        Self::Storage,
        Self::Keybinds,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Recording => "Recording",
            Self::Clips => "Clips & Review",
            Self::Storage => "Storage",
            Self::Keybinds => "Keybinds",
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Recording => "recording",
            Self::Clips => "clips",
            Self::Storage => "storage",
            Self::Keybinds => "keybinds",
        }
    }
}

impl Ui {
    pub(crate) fn open_settings(&mut self, section: SettingsSection) {
        self.settings_open = true;
        self.settings_section = section;
        self.settings_search.clear();
        self.stop_rebinding();
    }

    pub(crate) fn close_settings(&mut self) {
        self.settings_open = false;
        self.settings_search.clear();
        self.resolution_open = false;
        self.storage_open = false;
        self.stop_rebinding();
    }

    pub(crate) fn clear_settings_search_on_escape(&mut self, ev: &KeyEvent) -> bool {
        let escape = ev.key == Key::Escape;
        let consume = escape && !self.settings_search.text.is_empty();
        if consume {
            self.settings_search.clear();
        }
        consume
    }

    pub(crate) fn toggle_settings(&mut self) {
        if self.settings_open {
            self.close_settings();
        } else {
            self.open_settings(self.settings_section);
        }
    }

    pub(crate) fn pick_settings_section(&mut self, section: SettingsSection) {
        self.settings_section = section;
        self.settings_search = TextEdit::default();
        self.resolution_open = false;
        self.storage_open = false;
        self.stop_rebinding();
    }

    pub(crate) fn storage_used_bytes(&self) -> u64 {
        LibraryTotals::of(&self.recordings).bytes
    }
}
