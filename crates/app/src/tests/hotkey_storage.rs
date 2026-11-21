use crossbeam_channel::Receiver;
use greyscreen_core::settings::{Hotkey, Settings};
use greyscreen_engine::{EngineCmd, UiMsg};
use rune::{Env, Key, KeyEvent, Modifier, Mods, Runtime};

use super::{settle, tap, visible_texts};
use crate::ui::{Screen, Ui};
use crate::view;

fn settings_runtime() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (cmd, rx) = crossbeam_channel::unbounded();
    let ui = Ui::live(Settings::default(), cmd);
    let mut rt = Runtime::new(ui, view::root, (960.0, 2600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.on_key = Some(Ui::on_key);
    tap(&mut rt, "nav-settings");
    (rt, rx)
}

fn press(rt: &mut Runtime<Ui>, key: Key, mods: Mods) {
    rt.key(KeyEvent { key, mods });
    rt.frame(1.0 / 60.0);
}

fn ctrl() -> Mods {
    Mods {
        ctrl: true,
        cmd: true,
        ..Mods::default()
    }
}

fn texts(rt: &Runtime<Ui>) -> Vec<String> {
    visible_texts(rt)
}

fn has_text(rt: &Runtime<Ui>, wanted: &str) -> bool {
    texts(rt).iter().any(|t| t == wanted)
}

fn start_capture(rt: &mut Runtime<Ui>) {
    tap(rt, "hotkey-button");
}

fn applied_hotkey(rx: &Receiver<EngineCmd>) -> Option<Option<Hotkey>> {
    rx.try_iter().last().and_then(|cmd| match cmd {
        EngineCmd::ApplySettings(settings) => Some(settings.save_clip_hotkey),
        _ => None,
    })
}

fn hotkey(text: &str) -> Hotkey {
    text.parse().expect("hotkey")
}

#[test]
fn the_row_shows_the_default_hotkey() {
    let (rt, _rx) = settings_runtime();
    assert!(has_text(&rt, "Save clip hotkey"));
    assert!(has_text(&rt, "F9"));
    assert!(rt.find("hotkey-off").is_some());
}

#[test]
fn clicking_the_button_waits_for_a_key() {
    let (mut rt, _rx) = settings_runtime();
    start_capture(&mut rt);
    assert!(rt.state.hotkey_capture);
    assert!(has_text(&rt, "Press a key…"));
    assert!(rt.find("hotkey-off").is_none());
}

#[test]
fn the_next_function_key_becomes_the_hotkey() {
    let (mut rt, rx) = settings_runtime();
    start_capture(&mut rt);
    press(&mut rt, Key::Function(10), Mods::default());
    assert!(!rt.state.hotkey_capture);
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F10")));
    assert!(has_text(&rt, "F10"));
    assert_eq!(applied_hotkey(&rx), Some(Some(hotkey("F10"))));
}

#[test]
fn a_chord_is_recorded_with_its_modifiers() {
    let (mut rt, rx) = settings_runtime();
    start_capture(&mut rt);
    press(&mut rt, Key::Command('k'), ctrl());
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("Ctrl+K")));
    assert_eq!(applied_hotkey(&rx), Some(Some(hotkey("Ctrl+K"))));
    start_capture(&mut rt);
    let mods = Mods {
        alt: true,
        shift: true,
        ..Mods::default()
    };
    press(&mut rt, Key::Text("j".into()), mods);
    assert_eq!(
        rt.state.settings.save_clip_hotkey,
        Some(hotkey("Alt+Shift+J"))
    );
    assert!(has_text(&rt, "Alt+Shift+J"));
}

#[test]
fn space_and_navigation_keys_can_be_part_of_a_chord() {
    let (mut rt, _rx) = settings_runtime();
    start_capture(&mut rt);
    press(&mut rt, Key::Text(" ".into()), ctrl());
    assert_eq!(
        rt.state.settings.save_clip_hotkey,
        Some(hotkey("Ctrl+Space"))
    );
    start_capture(&mut rt);
    press(&mut rt, Key::PageUp, ctrl());
    assert_eq!(
        rt.state.settings.save_clip_hotkey,
        Some(hotkey("Ctrl+PageUp"))
    );
}

#[test]
fn a_bare_letter_is_refused_with_a_hint_and_capture_goes_on() {
    let (mut rt, rx) = settings_runtime();
    start_capture(&mut rt);
    press(&mut rt, Key::Text("k".into()), Mods::default());
    assert!(rt.state.hotkey_capture);
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F9")));
    assert!(
        texts(&rt)
            .iter()
            .any(|t| t.starts_with("Hold Ctrl, Alt or Shift"))
    );
    assert!(rx.try_recv().is_err());
    press(&mut rt, Key::Function(8), Mods::default());
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F8")));
    assert!(rt.find("hotkey-note").is_none());
}

#[test]
fn keys_that_cannot_be_hotkeys_are_refused() {
    let (mut rt, _rx) = settings_runtime();
    start_capture(&mut rt);
    press(&mut rt, Key::Enter, Mods::default());
    assert!(rt.state.hotkey_capture);
    assert!(texts(&rt).iter().any(|t| t.starts_with("That key can't")));
    let logo = Mods {
        logo: true,
        ..Mods::default()
    };
    press(&mut rt, Key::Function(5), logo);
    assert!(rt.state.hotkey_capture);
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F9")));
}

#[test]
fn a_modifier_press_alone_keeps_waiting() {
    let (mut rt, _rx) = settings_runtime();
    start_capture(&mut rt);
    let key = Key::Modifier {
        modifier: Modifier::ControlLeft,
        pressed: true,
    };
    press(&mut rt, key, ctrl());
    assert!(rt.state.hotkey_capture);
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F9")));
}

#[test]
fn escape_cancels_the_capture_and_keeps_the_hotkey() {
    let (mut rt, rx) = settings_runtime();
    start_capture(&mut rt);
    press(&mut rt, Key::Escape, Mods::default());
    assert!(!rt.state.hotkey_capture);
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F9")));
    assert!(has_text(&rt, "F9"));
    assert!(rx.try_recv().is_err());
}

#[test]
fn clicking_the_button_again_cancels() {
    let (mut rt, _rx) = settings_runtime();
    start_capture(&mut rt);
    start_capture(&mut rt);
    assert!(!rt.state.hotkey_capture);
    assert!(has_text(&rt, "F9"));
}

#[test]
fn leaving_settings_cancels_the_capture() {
    let (mut rt, _rx) = settings_runtime();
    start_capture(&mut rt);
    tap(&mut rt, "nav-status");
    assert_eq!(rt.state.screen, Screen::Status);
    assert!(!rt.state.hotkey_capture);
}

#[test]
fn keys_are_not_captured_unless_asked() {
    let (mut rt, rx) = settings_runtime();
    press(&mut rt, Key::Function(10), Mods::default());
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F9")));
    assert!(rx.try_recv().is_err());
}

#[test]
fn turning_the_hotkey_off_clears_it() {
    let (mut rt, rx) = settings_runtime();
    tap(&mut rt, "hotkey-off");
    assert_eq!(rt.state.settings.save_clip_hotkey, None);
    assert_eq!(applied_hotkey(&rx), Some(None));
    assert!(has_text(&rt, "Not set"));
    assert!(rt.find("hotkey-off").is_none());
    start_capture(&mut rt);
    press(&mut rt, Key::Function(7), Mods::default());
    assert_eq!(rt.state.settings.save_clip_hotkey, Some(hotkey("F7")));
}

#[test]
fn a_failed_registration_shows_a_muted_note_until_it_clears() {
    let (mut rt, _rx) = settings_runtime();
    let note = "F9 is already used by another app. Choose a different key.";
    rt.update(|ui, tx| {
        ui.apply(
            UiMsg::HotkeyStatus {
                error: Some(note.into()),
            },
            tx,
        );
    });
    settle(&mut rt);
    assert!(has_text(&rt, note));
    assert!(rt.find("hotkey-note").is_some());
    rt.update(|ui, tx| ui.apply(UiMsg::HotkeyStatus { error: None }, tx));
    settle(&mut rt);
    assert!(rt.find("hotkey-note").is_none());
    assert!(!has_text(&rt, note));
}

fn open_storage_menu(rt: &mut Runtime<Ui>) {
    tap(rt, "storage-limit-trigger");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn the_storage_limit_row_is_off_by_default_and_warns_about_permanent_deletes() {
    let (rt, _rx) = settings_runtime();
    assert!(has_text(&rt, "Storage limit"));
    assert!(has_text(&rt, "Off"));
    assert!(has_text(
        &rt,
        "Oldest full-game videos are deleted permanently when recordings pass this size. Clips are kept as long as possible."
    ));
}

#[test]
fn the_storage_menu_lists_off_and_the_four_limits() {
    let (mut rt, _rx) = settings_runtime();
    open_storage_menu(&mut rt);
    assert!(rt.state.storage_open);
    for label in ["50 GB", "100 GB", "250 GB", "500 GB"] {
        assert!(has_text(&rt, label), "missing {label}");
    }
}

#[test]
fn picking_a_limit_stores_it_and_tells_the_engine() {
    let (mut rt, rx) = settings_runtime();
    open_storage_menu(&mut rt);
    tap(&mut rt, "option3");
    assert!(!rt.state.storage_open);
    assert_eq!(rt.state.settings.storage_limit_gb, Some(250));
    match rx.try_iter().last() {
        Some(EngineCmd::ApplySettings(settings)) => {
            assert_eq!(settings.storage_limit_gb, Some(250));
        }
        other => panic!("expected the settings, got {other:?}"),
    }
    assert!(has_text(&rt, "250 GB"));
}

#[test]
fn picking_off_removes_the_limit() {
    let (mut rt, _rx) = settings_runtime();
    rt.update(|ui, _| ui.settings.storage_limit_gb = Some(100));
    rt.frame(1.0 / 60.0);
    assert!(has_text(&rt, "100 GB"));
    open_storage_menu(&mut rt);
    tap(&mut rt, "option0");
    assert_eq!(rt.state.settings.storage_limit_gb, None);
    assert!(has_text(&rt, "Off"));
}
