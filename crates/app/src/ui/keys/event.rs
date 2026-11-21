use greyscreen_core::keybinds::KeyCombo;
use rune::{Key, KeyEvent};

pub(crate) const MAC: bool = cfg!(target_os = "macos");

pub(crate) fn combo_of(ev: &KeyEvent) -> Option<KeyCombo> {
    let mods = ev.mods;
    let bare_primary = mods.cmd && !mods.ctrl && !mods.logo;
    let ctrl = mods.ctrl || (bare_primary && !MAC);
    let meta = mods.logo || (bare_primary && MAC);
    KeyCombo::new(ctrl, mods.alt, mods.shift, meta, &key_name(&ev.key)?)
}

fn key_name(key: &Key) -> Option<String> {
    Some(match key {
        Key::Function(n) => format!("F{n}"),
        _ if is_space(key) => "Space".to_owned(),
        Key::Text(text) => text.clone(),
        Key::Command(c) => c.to_string(),
        Key::Escape => "Escape".to_owned(),
        Key::Enter => "Enter".to_owned(),
        Key::Tab => "Tab".to_owned(),
        Key::Backspace => "Backspace".to_owned(),
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
