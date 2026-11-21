use greyscreen_core::settings::Hotkey;
use rune::{Key, KeyEvent, Tx};

use super::Ui;

const HINT_NEEDS_MODIFIER: &str = "Hold Ctrl, Alt or Shift with that key, or use a function key.";
const HINT_UNSUPPORTED: &str = "That key can't be a hotkey. Try another one.";

impl Ui {
    pub(crate) fn toggle_hotkey_capture(&mut self, tx: &mut Tx) {
        self.hotkey_hint = None;
        self.hotkey_capture = !self.hotkey_capture;
        if self.hotkey_capture {
            tx.blur();
        }
    }

    pub(crate) fn clear_hotkey(&mut self) {
        self.hotkey_capture = false;
        self.hotkey_hint = None;
        self.change(|s| s.save_clip_hotkey = None);
    }

    pub(super) fn capture_hotkey(&mut self, ev: &KeyEvent) -> bool {
        match &ev.key {
            Key::Modifier { .. } => return true,
            Key::Escape => {
                self.hotkey_capture = false;
                self.hotkey_hint = None;
                return true;
            }
            _ => {}
        }
        let mods = ev.mods;
        let Some(name) = key_name(&ev.key) else {
            self.hotkey_hint = Some(HINT_UNSUPPORTED);
            return true;
        };
        if mods.logo {
            self.hotkey_hint = Some(HINT_UNSUPPORTED);
            return true;
        }
        match Hotkey::new(mods.ctrl, mods.alt, mods.shift, &name) {
            Some(hotkey) => {
                self.hotkey_capture = false;
                self.hotkey_hint = None;
                self.change(|s| s.save_clip_hotkey = Some(hotkey));
            }
            None => self.hotkey_hint = Some(HINT_NEEDS_MODIFIER),
        }
        true
    }
}

fn key_name(key: &Key) -> Option<String> {
    Some(match key {
        Key::Function(n) => format!("F{n}"),
        _ if is_space(key) => "Space".to_owned(),
        Key::Text(text) => text.clone(),
        Key::Command(c) => c.to_string(),
        Key::Insert => "Insert".to_owned(),
        Key::Delete => "Delete".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::End => "End".to_owned(),
        Key::PageUp => "PageUp".to_owned(),
        Key::PageDown => "PageDown".to_owned(),
        Key::Left => "Left".to_owned(),
        Key::Right => "Right".to_owned(),
        Key::Up => "Up".to_owned(),
        Key::Down => "Down".to_owned(),
        Key::Pause => "Pause".to_owned(),
        _ => return None,
    })
}

fn is_space(key: &Key) -> bool {
    matches!(key, Key::Text(t) if t == " ") || matches!(key, Key::Command(' '))
}
