use super::{KeyCombo, combo};

#[test]
fn display_and_parse_round_trip() {
    for text in [
        "F9",
        "Alt+F9",
        "Ctrl+Shift+K",
        "Ctrl+Alt+Shift+Meta+7",
        "Ctrl+Space",
        "Alt+PageUp",
        "Left",
        "Shift+Right",
        "J",
        ",",
        "Ctrl+,",
        "[",
        "Escape",
        "Meta+1",
    ] {
        assert_eq!(combo(text).to_string(), text);
    }
}

#[test]
fn parse_ignores_case_and_spaces_and_knows_aliases() {
    assert_eq!(combo("ctrl + shift + k").to_string(), "Ctrl+Shift+K");
    assert_eq!(combo("f12").to_string(), "F12");
    assert_eq!(combo("CONTROL+pageup").to_string(), "Ctrl+PageUp");
    assert_eq!(combo("Cmd+K").to_string(), "Meta+K");
    assert_eq!(combo("Win+K").to_string(), "Meta+K");
    assert_eq!(combo("Option+K").to_string(), "Alt+K");
    assert_eq!(combo("space").to_string(), "Space");
}

#[test]
fn modifier_order_is_fixed() {
    assert_eq!(
        combo("Meta+Shift+Alt+Ctrl+A").to_string(),
        "Ctrl+Alt+Shift+Meta+A"
    );
}

#[test]
fn mod_is_the_platform_primary_key() {
    let primary = combo("Mod+,");
    if cfg!(target_os = "macos") {
        assert!(primary.meta && !primary.ctrl);
    } else {
        assert!(primary.ctrl && !primary.meta);
    }
    assert_eq!(KeyCombo::primary(","), Some(primary));
}

#[test]
fn nonsense_is_refused() {
    for text in [
        "",
        "Hyper+Q",
        "Ctrl+",
        "Ctrl+Shift",
        "KK",
        "F25",
        "F0",
        "Ctrl+Ctrl+Q+Q",
        "é",
    ] {
        assert!(text.parse::<KeyCombo>().is_err(), "{text:?}");
    }
}

#[test]
fn serde_uses_the_text_form() {
    let combo = combo("Ctrl+Shift+F9");
    let json = serde_json::to_string(&combo).unwrap();
    assert_eq!(json, "\"Ctrl+Shift+F9\"");
    assert_eq!(serde_json::from_str::<KeyCombo>(&json).unwrap(), combo);
    assert!(serde_json::from_str::<KeyCombo>("\"Hyper+Q\"").is_err());
}

#[test]
fn display_follows_the_platform() {
    let chord = combo("Ctrl+Alt+Shift+Meta+Left");
    assert_eq!(chord.display(false), "Ctrl+Alt+Shift+Win+←");
    assert_eq!(chord.display(true), "Ctrl+Option+Shift+Cmd+←");
    assert_eq!(combo("F9").keycaps(false), vec!["F9"]);
}

#[test]
fn global_hotkeys_need_a_modifier_or_a_function_key() {
    assert!(combo("F9").usable_as_global());
    assert!(combo("Shift+K").usable_as_global());
    assert!(combo("Ctrl+Space").usable_as_global());
    for text in ["K", "7", "Space", "Left", ","] {
        assert!(!combo(text).usable_as_global(), "{text}");
    }
}

#[test]
fn virtual_keys_follow_windows_codes() {
    assert_eq!(combo("F9").virtual_key(), 0x78);
    assert_eq!(combo("F1").virtual_key(), 0x70);
    assert_eq!(combo("K").virtual_key(), u32::from(b'K'));
    assert_eq!(combo("7").virtual_key(), u32::from(b'7'));
    assert_eq!(combo("Space").virtual_key(), 0x20);
    assert_eq!(combo("Escape").virtual_key(), 0x1B);
    assert_eq!(combo(",").virtual_key(), 0xBC);
    assert_eq!(combo("]").virtual_key(), 0xDD);
}
