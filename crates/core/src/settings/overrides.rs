use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer};

use crate::keybinds::{Action, KeyCombo};

pub(super) fn lenient_overrides<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    let raw = serde_json::Value::deserialize(deserializer)?;
    let serde_json::Value::Object(map) = raw else {
        return Ok(BTreeMap::new());
    };
    Ok(map
        .into_iter()
        .filter_map(|(id, value)| Some((id, value.as_str()?.to_owned())))
        .collect())
}

pub(super) fn legacy_hotkey<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

pub(super) fn migrate_legacy(
    legacy: &mut Option<Option<String>>,
    overrides: &mut BTreeMap<String, String>,
) {
    let Some(old) = legacy.take() else {
        return;
    };
    let id = Action::SaveClip.id();
    if overrides.contains_key(id) {
        return;
    }
    match old {
        None => {
            overrides.insert(id.to_owned(), "none".to_owned());
        }
        Some(text) => {
            if let Ok(combo) = text.parse::<KeyCombo>()
                && combo.usable_as_global()
            {
                overrides.insert(id.to_owned(), combo.to_string());
            }
        }
    }
}
