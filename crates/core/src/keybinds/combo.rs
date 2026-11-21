use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const FUNCTION_KEYS: u8 = 24;
const VK_F1: u32 = 0x70;

const NAMED_KEYS: [(&str, u32); 16] = [
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
    ("Escape", 0x1B),
    ("Enter", 0x0D),
    ("Tab", 0x09),
    ("Backspace", 0x08),
];

const PUNCTUATION: [(char, u32); 11] = [
    (',', 0xBC),
    ('.', 0xBE),
    ('-', 0xBD),
    ('=', 0xBB),
    (';', 0xBA),
    ('/', 0xBF),
    ('`', 0xC0),
    ('[', 0xDB),
    ('\\', 0xDC),
    (']', 0xDD),
    ('\'', 0xDE),
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    key: String,
}

impl KeyCombo {
    pub fn new(ctrl: bool, alt: bool, shift: bool, meta: bool, key: &str) -> Option<KeyCombo> {
        Some(KeyCombo {
            ctrl,
            alt,
            shift,
            meta,
            key: canonical_key(key)?,
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn primary(key: &str) -> Option<KeyCombo> {
        let mac = cfg!(target_os = "macos");
        KeyCombo::new(!mac, false, false, mac, key)
    }

    pub fn usable_as_global(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.meta || is_function_key(&self.key)
    }

    pub fn virtual_key(&self) -> u32 {
        let mut chars = self.key.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_alphanumeric() => u32::from(c),
            (Some(c), None) => PUNCTUATION
                .iter()
                .find(|(p, _)| *p == c)
                .map_or(0, |(_, code)| *code),
            (Some('F'), Some(_)) => self.key[1..]
                .parse::<u32>()
                .map_or(0, |n| VK_F1 + n.saturating_sub(1)),
            _ => NAMED_KEYS
                .iter()
                .find(|(name, _)| *name == self.key)
                .map_or(0, |(_, code)| *code),
        }
    }

    pub fn keycaps(&self, mac: bool) -> Vec<String> {
        let mut caps = Vec::new();
        for (held, name) in [
            (self.ctrl, "Ctrl"),
            (self.alt, if mac { "Option" } else { "Alt" }),
            (self.shift, "Shift"),
            (self.meta, if mac { "Cmd" } else { "Win" }),
        ] {
            if held {
                caps.push(name.to_owned());
            }
        }
        caps.push(
            match self.key.as_str() {
                "Left" => "←",
                "Right" => "→",
                "Up" => "↑",
                "Down" => "↓",
                other => other,
            }
            .to_owned(),
        );
        caps
    }

    pub fn display(&self, mac: bool) -> String {
        self.keycaps(mac).join("+")
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
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase().to_string());
        }
        return PUNCTUATION
            .iter()
            .any(|(p, _)| *p == c)
            .then(|| c.to_string());
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

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [
            (self.ctrl, "Ctrl"),
            (self.alt, "Alt"),
            (self.shift, "Shift"),
            (self.meta, "Meta"),
        ] {
            if held {
                write!(f, "{name}+")?;
            }
        }
        f.write_str(&self.key)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("not a usable key combination: {0}")]
pub struct ParseKeyComboError(String);

impl FromStr for KeyCombo {
    type Err = ParseKeyComboError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = || ParseKeyComboError(text.to_owned());
        let mac = cfg!(target_os = "macos");
        let (mut ctrl, mut alt, mut shift, mut meta) = (false, false, false, false);
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let key = parts.pop().ok_or_else(bad)?;
        for part in parts {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" | "option" => alt = true,
                "shift" => shift = true,
                "meta" | "cmd" | "command" | "win" | "super" => meta = true,
                "mod" | "primary" if mac => meta = true,
                "mod" | "primary" => ctrl = true,
                _ => return Err(bad()),
            }
        }
        KeyCombo::new(ctrl, alt, shift, meta, key).ok_or_else(bad)
    }
}

impl Serialize for KeyCombo {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for KeyCombo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}
