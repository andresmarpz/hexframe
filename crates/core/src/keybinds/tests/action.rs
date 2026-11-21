use std::collections::HashSet;

use crate::keybinds::{Action, Group, Keymap, Scope};

#[test]
fn ids_are_unique_and_found_again() {
    let mut seen = HashSet::new();
    for action in Action::ALL {
        assert!(seen.insert(action.id()), "{}", action.id());
        assert_eq!(Action::from_id(action.id()), Some(*action));
    }
    assert_eq!(Action::from_id("nope"), None);
}

#[test]
fn every_default_parses() {
    for action in Action::ALL {
        for text in action.spec().defaults {
            assert!(text.parse::<crate::keybinds::KeyCombo>().is_ok(), "{text}");
        }
    }
}

#[test]
fn the_defaults_have_no_conflicts() {
    let keymap = Keymap::defaults();
    for action in Action::ALL {
        assert!(keymap.conflicts(*action).is_empty(), "{}", action.id());
    }
}

#[test]
fn global_defaults_can_be_registered_without_a_window() {
    for action in Action::ALL {
        if action.scope() == Scope::Global {
            for combo in Keymap::defaults().bindings(*action) {
                assert!(combo.usable_as_global(), "{}", action.id());
            }
        }
    }
}

#[test]
fn every_group_has_an_action() {
    for group in Group::ALL {
        assert!(Action::ALL.iter().any(|a| a.group() == group));
    }
}
