mod action;
mod combo;
mod keymap;

use super::KeyCombo;

fn combo(text: &str) -> KeyCombo {
    text.parse().unwrap()
}
