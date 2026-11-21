use greyscreen_core::keybinds::{Action, BindError, KeyCombo};
use rune::{Key, KeyEvent, Tx};

use super::Ui;
use super::key_event::{MAC, combo_of};

const HINT_UNSUPPORTED: &str = "That key can't be used. Try another one.";

impl Ui {
    pub(crate) fn toggle_rebind(&mut self, action: Action, tx: &mut Tx) {
        self.bind_hint = None;
        if self.rebinding == Some(action) {
            self.rebinding = None;
            return;
        }
        self.rebinding = Some(action);
        tx.blur();
    }

    pub(crate) fn stop_rebinding(&mut self) {
        self.rebinding = None;
        self.bind_hint = None;
    }

    pub(crate) fn reset_binding(&mut self, action: Action) {
        let mut keymap = self.settings.keymap();
        keymap.reset(action);
        self.store_keymap(&keymap);
    }

    pub(crate) fn reset_all_bindings(&mut self) {
        let mut keymap = self.settings.keymap();
        keymap.reset_all();
        self.store_keymap(&keymap);
    }

    pub(super) fn capture_binding(&mut self, ev: &KeyEvent) -> bool {
        let Some(action) = self.rebinding else {
            return false;
        };
        match &ev.key {
            Key::Modifier { .. } => return true,
            Key::Escape => {
                self.stop_rebinding();
                return true;
            }
            Key::Backspace => {
                self.bind(action, None);
                return true;
            }
            _ => {}
        }
        match combo_of(ev) {
            Some(combo) => self.bind(action, Some(combo)),
            None => self.bind_hint = Some((action, HINT_UNSUPPORTED.to_owned())),
        }
        true
    }

    fn bind(&mut self, action: Action, combo: Option<KeyCombo>) {
        let mut keymap = self.settings.keymap();
        let shown = combo.clone();
        match keymap.bind(action, combo) {
            Ok(()) => {
                self.stop_rebinding();
                self.store_keymap(&keymap);
            }
            Err(BindError::Conflict(other)) => {
                let key = shown.map_or_else(String::new, |c| c.display(MAC));
                let text = format!("{key} is already used by {}.", other.label());
                self.bind_hint = Some((action, text));
            }
            Err(error) => self.bind_hint = Some((action, error.to_string())),
        }
    }

    fn store_keymap(&mut self, keymap: &greyscreen_core::keybinds::Keymap) {
        let mut overrides = self.settings.keybinds.clone();
        keymap.store(&mut overrides);
        self.change(|s| s.keybinds = overrides);
    }
}
