use super::*;

fn key(text: &str) -> Hotkey {
    text.parse().unwrap()
}

#[test]
fn the_default_is_a_bare_f9() {
    let hotkey = Hotkey::default();
    assert_eq!(hotkey.to_string(), "F9");
    assert_eq!(hotkey.virtual_key(), 0x78);
}

#[test]
fn display_and_parse_round_trip() {
    for text in [
        "F9",
        "Alt+F9",
        "Ctrl+Shift+K",
        "Ctrl+Alt+Shift+7",
        "Ctrl+Space",
        "Alt+PageUp",
    ] {
        assert_eq!(key(text).to_string(), text);
    }
}

#[test]
fn parse_ignores_case_and_spaces() {
    assert_eq!(key("ctrl + shift + k").to_string(), "Ctrl+Shift+K");
    assert_eq!(key("f12").to_string(), "F12");
    assert_eq!(key("CONTROL+pageup").to_string(), "Ctrl+PageUp");
}

#[test]
fn modifier_order_is_fixed() {
    assert_eq!(key("Shift+Alt+Ctrl+A").to_string(), "Ctrl+Alt+Shift+A");
}

#[test]
fn a_bare_typing_key_is_refused() {
    for text in ["K", "7", "Space", "Left", "Delete", "Insert"] {
        assert!(text.parse::<Hotkey>().is_err(), "{text}");
    }
}

#[test]
fn a_bare_function_key_is_allowed() {
    for n in 1..=24 {
        assert!(Hotkey::new(false, false, false, &format!("F{n}")).is_some());
    }
}

#[test]
fn unknown_keys_and_modifiers_are_refused() {
    for text in [
        "",
        "+",
        "Ctrl+",
        "F0",
        "F25",
        "Win+K",
        "Ctrl+Enter",
        "Ctrl+Esc",
        "Ctrl+ab",
    ] {
        assert!(text.parse::<Hotkey>().is_err(), "{text:?}");
    }
}

#[test]
fn virtual_key_codes_match_windows() {
    assert_eq!(key("F1").virtual_key(), 0x70);
    assert_eq!(key("F24").virtual_key(), 0x87);
    assert_eq!(key("Ctrl+A").virtual_key(), 0x41);
    assert_eq!(key("Ctrl+Z").virtual_key(), 0x5A);
    assert_eq!(key("Ctrl+0").virtual_key(), 0x30);
    assert_eq!(key("Ctrl+9").virtual_key(), 0x39);
    assert_eq!(key("Ctrl+Space").virtual_key(), 0x20);
    assert_eq!(key("Alt+Delete").virtual_key(), 0x2E);
    assert_eq!(key("Alt+Down").virtual_key(), 0x28);
    assert_eq!(key("Alt+Pause").virtual_key(), 0x13);
}

#[test]
fn every_named_key_has_a_code() {
    for (name, code) in NAMED_KEYS {
        assert_eq!(key(&format!("Ctrl+{name}")).virtual_key(), code);
    }
}

#[test]
fn serde_uses_the_display_text() {
    let json = serde_json::to_string(&key("Ctrl+Shift+F9")).unwrap();
    assert_eq!(json, "\"Ctrl+Shift+F9\"");
}
