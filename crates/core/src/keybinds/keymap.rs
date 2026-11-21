use std::collections::BTreeMap;

use super::{Action, KeyCombo, Scope};

const NONE: &str = "none";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BindError {
    #[error("this key is fixed")]
    Fixed,
    #[error("Hold Ctrl, Alt or Shift with that key, or use a function key.")]
    NeedsModifier,
    #[error("already used by {}", .0.label())]
    Conflict(Action),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    bindings: BTreeMap<Action, Vec<KeyCombo>>,
}

impl Keymap {
    pub fn defaults() -> Keymap {
        Keymap {
            bindings: Action::ALL
                .iter()
                .map(|action| (*action, default_bindings(*action)))
                .collect(),
        }
    }

    pub fn from_overrides(overrides: &BTreeMap<String, String>) -> Keymap {
        let mut keymap = Keymap::defaults();
        for action in Action::ALL {
            if action.spec().fixed {
                continue;
            }
            let Some(text) = overrides.get(action.id()) else {
                continue;
            };
            if text.trim().eq_ignore_ascii_case(NONE) {
                keymap.bindings.insert(*action, Vec::new());
            } else if let Ok(combo) = text.parse::<KeyCombo>() {
                keymap.bindings.insert(*action, vec![combo]);
            }
        }
        keymap
    }

    pub fn store(&self, overrides: &mut BTreeMap<String, String>) {
        for action in Action::ALL {
            if action.spec().fixed {
                continue;
            }
            if self.is_default(*action) {
                overrides.remove(action.id());
                continue;
            }
            let text = match self.bindings(*action) {
                [] => NONE.to_owned(),
                [first, ..] => first.to_string(),
            };
            overrides.insert(action.id().to_owned(), text);
        }
    }

    pub fn bindings(&self, action: Action) -> &[KeyCombo] {
        self.bindings.get(&action).map_or(&[], Vec::as_slice)
    }

    pub fn is_default(&self, action: Action) -> bool {
        self.bindings(action) == default_bindings(action)
    }

    pub fn action_for(&self, scope: Scope, combo: &KeyCombo) -> Option<Action> {
        self.bindings
            .iter()
            .find(|(action, combos)| action.scope() == scope && combos.contains(combo))
            .map(|(action, _)| *action)
    }

    pub fn global(&self) -> Vec<(Action, KeyCombo)> {
        self.bindings
            .iter()
            .filter(|(action, _)| action.scope() == Scope::Global)
            .flat_map(|(action, combos)| combos.iter().map(|combo| (*action, combo.clone())))
            .collect()
    }

    pub fn conflicts(&self, action: Action) -> Vec<Action> {
        let mine = self.bindings(action);
        self.bindings
            .iter()
            .filter(|(other, combos)| {
                **other != action && combos.iter().any(|combo| mine.contains(combo))
            })
            .map(|(other, _)| *other)
            .collect()
    }

    pub fn check(&self, action: Action, combo: &KeyCombo) -> Result<(), BindError> {
        if action.spec().fixed {
            return Err(BindError::Fixed);
        }
        if action.scope() == Scope::Global && !combo.usable_as_global() {
            return Err(BindError::NeedsModifier);
        }
        match self
            .bindings
            .iter()
            .find(|(other, combos)| **other != action && combos.contains(combo))
        {
            Some((other, _)) => Err(BindError::Conflict(*other)),
            None => Ok(()),
        }
    }

    pub fn bind(&mut self, action: Action, combo: Option<KeyCombo>) -> Result<(), BindError> {
        let bindings = match combo {
            Some(combo) => {
                self.check(action, &combo)?;
                vec![combo]
            }
            None if action.spec().fixed => return Err(BindError::Fixed),
            None => Vec::new(),
        };
        self.bindings.insert(action, bindings);
        Ok(())
    }

    pub fn reset(&mut self, action: Action) {
        self.bindings.insert(action, default_bindings(action));
    }

    pub fn reset_all(&mut self) {
        *self = Keymap::defaults();
    }
}

fn default_bindings(action: Action) -> Vec<KeyCombo> {
    action
        .spec()
        .defaults
        .iter()
        .filter_map(|text| text.parse().ok())
        .collect()
}
