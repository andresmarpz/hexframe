use std::collections::BTreeMap;

use super::combo;
use crate::keybinds::{Action, BindError, Keymap, Scope};

fn overrides(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(id, text)| ((*id).to_owned(), (*text).to_owned()))
        .collect()
}

#[test]
fn defaults_map_keys_to_actions_per_scope() {
    let keymap = Keymap::defaults();
    assert_eq!(
        keymap.action_for(Scope::Global, &combo("F9")),
        Some(Action::SaveClip)
    );
    assert_eq!(keymap.action_for(Scope::App, &combo("F9")), None);
    assert_eq!(
        keymap.action_for(Scope::App, &combo("Space")),
        Some(Action::PlayPause)
    );
    assert_eq!(keymap.action_for(Scope::Global, &combo("Space")), None);
    assert_eq!(
        keymap.action_for(Scope::App, &combo("Escape")),
        Some(Action::ExitFullscreen)
    );
}

#[test]
fn an_action_can_have_two_default_keys() {
    let keymap = Keymap::defaults();
    assert_eq!(keymap.bindings(Action::SeekBack).len(), 2);
    for key in ["Left", "J"] {
        assert_eq!(
            keymap.action_for(Scope::App, &combo(key)),
            Some(Action::SeekBack)
        );
    }
}

#[test]
fn overrides_replace_defaults_and_none_unbinds() {
    let keymap = Keymap::from_overrides(&overrides(&[
        ("save_clip", "Ctrl+Shift+K"),
        ("mute", "None"),
        ("play_pause", "P"),
    ]));
    assert_eq!(
        keymap.global(),
        vec![(Action::SaveClip, combo("Ctrl+Shift+K"))]
    );
    assert!(keymap.bindings(Action::Mute).is_empty());
    assert_eq!(keymap.action_for(Scope::App, &combo("M")), None);
    assert_eq!(keymap.action_for(Scope::App, &combo("Space")), None);
    assert_eq!(
        keymap.action_for(Scope::App, &combo("P")),
        Some(Action::PlayPause)
    );
}

#[test]
fn unknown_ids_and_bad_values_are_ignored_and_missing_ones_use_defaults() {
    let keymap = Keymap::from_overrides(&overrides(&[
        ("teleport", "T"),
        ("mute", "Hyper+Q"),
        ("exit_fullscreen", "none"),
    ]));
    assert_eq!(keymap, Keymap::defaults());
}

#[test]
fn store_keeps_only_changes_and_unknown_ids() {
    let mut map = overrides(&[("teleport", "T"), ("mute", "N")]);
    let mut keymap = Keymap::from_overrides(&map);
    keymap.bind(Action::SaveClip, None).unwrap();
    keymap.reset(Action::Mute);
    keymap.store(&mut map);
    assert_eq!(map, overrides(&[("teleport", "T"), ("save_clip", "none")]));
    assert_eq!(Keymap::from_overrides(&map), keymap);
}

#[test]
fn binding_replaces_every_default_of_the_action() {
    let mut keymap = Keymap::defaults();
    keymap.bind(Action::SeekBack, Some(combo("A"))).unwrap();
    assert_eq!(keymap.bindings(Action::SeekBack), [combo("A")]);
    assert_eq!(keymap.action_for(Scope::App, &combo("Left")), None);
    assert!(!keymap.is_default(Action::SeekBack));
}

#[test]
fn a_key_used_twice_is_refused_in_the_same_scope() {
    let mut keymap = Keymap::defaults();
    assert_eq!(
        keymap.bind(Action::Mute, Some(combo("F"))),
        Err(BindError::Conflict(Action::ToggleFullscreen))
    );
    assert_eq!(keymap.bindings(Action::Mute), [combo("M")]);
    assert_eq!(keymap.bind(Action::Mute, Some(combo("M"))), Ok(()));
}

#[test]
fn an_app_key_equal_to_a_global_one_is_refused_both_ways() {
    let mut keymap = Keymap::defaults();
    assert_eq!(
        keymap.bind(Action::Mute, Some(combo("F9"))),
        Err(BindError::Conflict(Action::SaveClip))
    );
    assert_eq!(
        keymap.bind(Action::SaveClip, Some(combo("Shift+Left"))),
        Err(BindError::Conflict(Action::PreviousEvent))
    );
}

#[test]
fn a_global_key_needs_a_modifier_unless_it_is_a_function_key() {
    let mut keymap = Keymap::defaults();
    assert_eq!(
        keymap.bind(Action::SaveClip, Some(combo("K"))),
        Err(BindError::NeedsModifier)
    );
    assert_eq!(keymap.bind(Action::SaveClip, Some(combo("F10"))), Ok(()));
    assert_eq!(keymap.bind(Action::SaveClip, Some(combo("Ctrl+K"))), Ok(()));
    assert_eq!(keymap.bind(Action::Mute, Some(combo("K"))), Ok(()));
}

#[test]
fn a_fixed_action_cannot_change() {
    let mut keymap = Keymap::defaults();
    assert_eq!(
        keymap.bind(Action::ExitFullscreen, Some(combo("Q"))),
        Err(BindError::Fixed)
    );
    assert_eq!(
        keymap.bind(Action::ExitFullscreen, None),
        Err(BindError::Fixed)
    );
    assert_eq!(keymap, Keymap::defaults());
}

#[test]
fn conflicts_found_in_a_hand_edited_file_are_reported_for_both_actions() {
    let keymap = Keymap::from_overrides(&overrides(&[("mute", "F")]));
    assert_eq!(
        keymap.conflicts(Action::Mute),
        vec![Action::ToggleFullscreen]
    );
    assert_eq!(
        keymap.conflicts(Action::ToggleFullscreen),
        vec![Action::Mute]
    );
    assert!(keymap.conflicts(Action::PlayPause).is_empty());
}

#[test]
fn reset_one_and_reset_all_restore_the_defaults() {
    let mut keymap = Keymap::defaults();
    keymap.bind(Action::Mute, Some(combo("N"))).unwrap();
    keymap.bind(Action::SaveClip, None).unwrap();
    keymap.reset(Action::Mute);
    assert!(keymap.is_default(Action::Mute));
    assert!(!keymap.is_default(Action::SaveClip));
    keymap.reset_all();
    assert_eq!(keymap, Keymap::defaults());
}

#[test]
fn only_global_bindings_are_listed_for_registration() {
    let mut keymap = Keymap::defaults();
    assert_eq!(keymap.global(), vec![(Action::SaveClip, combo("F9"))]);
    keymap.bind(Action::SaveClip, None).unwrap();
    assert!(keymap.global().is_empty());
}
