use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const FUNCTION_KEYS: u8 = 24;
const VK_F1: u32 = 0x70;

const NAMED_KEYS: [(&str, u32); 12] = [
    ("Space", 0x20),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("End", 0x23),
    ("Home", 0x24),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Pause", 0x13),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    key: String,
}

impl Hotkey {
    pub const DEFAULT_KEY: &'static str = "F9";

    pub fn new(ctrl: bool, alt: bool, shift: bool, key: &str) -> Option<Hotkey> {
        let key = canonical_key(key)?;
        let bare_ok = is_function_key(&key);
        if !(ctrl || alt || shift || bare_ok) {
            return None;
        }
        Some(Hotkey {
            ctrl,
            alt,
            shift,
            key,
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn virtual_key(&self) -> u32 {
        let mut chars = self.key.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_alphanumeric() => u32::from(c),
            (Some('F'), Some(_)) => self.key[1..]
                .parse::<u32>()
                .map_or(0, |n| VK_F1 + n.saturating_sub(1)),
            _ => NAMED_KEYS
                .iter()
                .find(|(name, _)| *name == self.key)
                .map_or(0, |(_, code)| *code),
        }
    }
}

impl Default for Hotkey {
    fn default() -> Self {
        Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            key: Self::DEFAULT_KEY.to_owned(),
        }
    }
}

fn is_function_key(key: &str) -> bool {
    key.strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=FUNCTION_KEYS).contains(&n))
}

fn canonical_key(key: &str) -> Option<String> {
    let key = key.trim();
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return c
            .is_ascii_alphanumeric()
            .then(|| c.to_ascii_uppercase().to_string());
    }
    if let Some(name) = NAMED_KEYS
        .iter()
        .map(|(name, _)| *name)
        .find(|name| name.eq_ignore_ascii_case(key))
    {
        return Some(name.to_owned());
    }
    let upper = key.to_ascii_uppercase();
    is_function_key(&upper).then_some(upper)
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [
            (self.ctrl, "Ctrl"),
            (self.alt, "Alt"),
            (self.shift, "Shift"),
        ] {
            if held {
                write!(f, "{name}+")?;
            }
        }
        f.write_str(&self.key)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("not a usable hotkey: {0}")]
pub struct ParseHotkeyError(String);

impl FromStr for Hotkey {
    type Err = ParseHotkeyError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = || ParseHotkeyError(text.to_owned());
        let (mut ctrl, mut alt, mut shift) = (false, false, false);
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let key = parts.pop().ok_or_else(bad)?;
        for part in parts {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                _ => return Err(bad()),
            }
        }
        Hotkey::new(ctrl, alt, shift, key).ok_or_else(bad)
    }
}

impl Serialize for Hotkey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

pub(super) fn deserialize_lenient<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Hotkey>, D::Error> {
    let text = Option::<String>::deserialize(deserializer)?;
    Ok(text.map(|text| text.parse().unwrap_or_default()))
}

#[cfg(test)]
mod tests;
