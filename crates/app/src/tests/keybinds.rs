#![allow(clippy::expect_used)]

use crossbeam_channel::Receiver;
use greyscreen_core::keybinds::{Action, KeyCombo};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, UiMsg};
use rune::{Env, Key, KeyEvent, Modifier, Mods, Runtime};

use super::{key_review_runtime, press_char, settle, tap, visible_texts};
use crate::ui::fake_player::{self, Call};
use crate::ui::{Detail, SettingsSection, Ui};
use crate::view;

fn keybinds_runtime() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (cmd, rx) = crossbeam_channel::unbounded();
    let ui = Ui::live(Settings::default(), cmd);
    let mut rt = Runtime::new(ui, view::root, (960.0, 2600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| {
        ui.open_settings(SettingsSection::Keybinds);
    });
    rt.frame(1.0 / 60.0);
    rx.try_iter().for_each(drop);
    (rt, rx)
}

fn press_with(rt: &mut Runtime<Ui>, key: Key, mods: Mods) {
    rt.key(KeyEvent { key, mods });
    rt.frame(1.0 / 60.0);
}

fn press_key(rt: &mut Runtime<Ui>, key: Key) {
    press_with(rt, key, Mods::default());
}

fn ctrl() -> Mods {
    Mods {
        ctrl: true,
        cmd: true,
        ..Mods::default()
    }
}

fn has_text(rt: &Runtime<Ui>, wanted: &str) -> bool {
    visible_texts(rt).iter().any(|t| t == wanted)
}

fn has_prefix(rt: &Runtime<Ui>, prefix: &str) -> bool {
    visible_texts(rt).iter().any(|t| t.starts_with(prefix))
}

fn combo(text: &str) -> KeyCombo {
    text.parse().expect("combo")
}

fn start_rebind(rt: &mut Runtime<Ui>, action: Action) {
    tap(rt, &format!("bind-btn-{}", action.id()));
}

fn bindings(rt: &Runtime<Ui>, action: Action) -> Vec<KeyCombo> {
    rt.state.settings.keymap().bindings(action).to_vec()
}

fn last_applied(rx: &Receiver<EngineCmd>) -> Option<Settings> {
    rx.try_iter().last().and_then(|cmd| match cmd {
        EngineCmd::ApplySettings(settings) => Some(settings),
        _ => None,
    })
}

#[test]
fn the_page_lists_every_action_under_its_group_with_the_default_keys() {
    let (rt, _rx) = keybinds_runtime();
    for group in ["Recording", "Review", "General"] {
        assert!(has_text(&rt, group), "{group}");
    }
    for action in Action::ALL {
        assert!(
            rt.find(&format!("bind-{}", action.id())).is_some(),
            "{}",
            action.id()
        );
        assert!(has_text(&rt, action.label()), "{}", action.label());
    }
    assert!(has_text(&rt, "Works in game"));
    assert!(has_text(&rt, "F9"));
    assert!(has_text(&rt, "Space"));
    assert!(rt.find("bind-reset-save_clip").is_none());
    assert!(rt.find("bind-btn-exit_fullscreen").is_none());
    assert!(rt.find("bind-fixed-exit_fullscreen").is_some());
}

#[test]
fn the_section_list_moves_between_recording_and_keybinds() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    tap(&mut rt, "settings-nav-recording");
    assert_eq!(rt.state.settings_section, SettingsSection::Recording);
    assert!(rt.state.rebinding.is_none());
    assert!(rt.find("recording-card").is_some());
    tap(&mut rt, "settings-nav-keybinds");
    assert_eq!(rt.state.settings_section, SettingsSection::Keybinds);
    assert!(rt.find("settings-keybinds").is_some());
}

#[test]
fn pressing_change_waits_for_keys() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    assert_eq!(rt.state.rebinding, Some(Action::Mute));
    assert!(has_text(&rt, "Press keys…"));
    assert!(has_text(
        &rt,
        "Press the keys to use. Esc cancels, Backspace clears."
    ));
    start_rebind(&mut rt, Action::Mute);
    assert!(rt.state.rebinding.is_none());
    assert!(!has_text(&rt, "Press keys…"));
}

#[test]
fn the_next_key_becomes_the_binding_and_reaches_the_engine() {
    let (mut rt, rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_char(&mut rt, "y");
    assert!(rt.state.rebinding.is_none());
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("Y")]);
    assert!(has_text(&rt, "Y"));
    assert!(rt.find("bind-reset-mute").is_some());
    let sent = last_applied(&rx).expect("settings sent");
    assert_eq!(sent.keybinds.get("mute").map(String::as_str), Some("Y"));
}

#[test]
fn a_combination_is_recorded_with_its_modifiers() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_with(&mut rt, Key::Command('k'), ctrl());
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("Ctrl+K")]);
    start_rebind(&mut rt, Action::Mute);
    let mods = Mods {
        alt: true,
        shift: true,
        ..Mods::default()
    };
    press_with(&mut rt, Key::Text("j".into()), mods);
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("Alt+Shift+J")]);
    start_rebind(&mut rt, Action::Mute);
    press_with(&mut rt, Key::Text(" ".into()), ctrl());
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("Ctrl+Space")]);
    assert!(has_text(&rt, "Ctrl Space"));
}

#[test]
fn home_and_function_keys_can_be_bound() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_key(&mut rt, Key::Home);
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("Home")]);
    start_rebind(&mut rt, Action::Mute);
    press_key(&mut rt, Key::Function(6));
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("F6")]);
}

#[test]
fn escape_cancels_and_keeps_the_binding() {
    let (mut rt, rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_key(&mut rt, Key::Escape);
    assert!(rt.state.rebinding.is_none());
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("M")]);
    assert!(rx.try_recv().is_err());
}

#[test]
fn backspace_clears_the_binding() {
    let (mut rt, rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_key(&mut rt, Key::Backspace);
    assert!(bindings(&rt, Action::Mute).is_empty());
    assert!(has_text(&rt, "Not set"));
    let sent = last_applied(&rx).expect("settings sent");
    assert_eq!(sent.keybinds.get("mute").map(String::as_str), Some("none"));
    start_rebind(&mut rt, Action::Mute);
    press_char(&mut rt, "y");
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("Y")]);
}

#[test]
fn a_taken_key_is_refused_with_a_note_and_capture_goes_on() {
    let (mut rt, rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_char(&mut rt, "f");
    assert_eq!(rt.state.rebinding, Some(Action::Mute));
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("M")]);
    assert!(has_text(&rt, "F is already used by Toggle fullscreen."));
    assert!(rx.try_recv().is_err());
    press_char(&mut rt, "y");
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("Y")]);
    assert!(!has_prefix(&rt, "F is already"));
}

#[test]
fn an_app_key_equal_to_the_global_one_is_refused() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_key(&mut rt, Key::Function(9));
    assert!(has_text(&rt, "F9 is already used by Save clip."));
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("M")]);
}

#[test]
fn a_global_key_needs_a_modifier_or_a_function_key() {
    let (mut rt, rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::SaveClip);
    press_char(&mut rt, "k");
    assert_eq!(rt.state.rebinding, Some(Action::SaveClip));
    assert!(has_prefix(&rt, "Hold Ctrl, Alt or Shift"));
    assert!(rx.try_recv().is_err());
    press_key(&mut rt, Key::Function(10));
    assert_eq!(bindings(&rt, Action::SaveClip), vec![combo("F10")]);
    let sent = last_applied(&rx).expect("settings sent");
    assert_eq!(
        sent.keymap().global(),
        vec![(Action::SaveClip, combo("F10"))]
    );
}

#[test]
fn a_global_binding_change_reaches_the_engine_as_the_registration_list() {
    let (mut rt, rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::SaveClip);
    press_with(&mut rt, Key::Command('k'), ctrl());
    let sent = last_applied(&rx).expect("settings sent");
    assert_eq!(
        sent.keymap().global(),
        vec![(Action::SaveClip, combo("Ctrl+K"))]
    );
    start_rebind(&mut rt, Action::SaveClip);
    press_key(&mut rt, Key::Backspace);
    assert!(
        last_applied(&rx)
            .expect("settings sent")
            .keymap()
            .global()
            .is_empty()
    );
}

#[test]
fn keys_that_cannot_be_bound_are_refused_with_a_hint() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_key(&mut rt, Key::ContextMenu);
    press_with(&mut rt, Key::Text("<".into()), Mods::default());
    assert_eq!(rt.state.rebinding, Some(Action::Mute));
    assert!(has_text(&rt, "That key can't be used. Try another one."));
}

#[test]
fn a_modifier_press_alone_keeps_waiting() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    let key = Key::Modifier {
        modifier: Modifier::ControlLeft,
        pressed: true,
    };
    press_with(&mut rt, key, ctrl());
    assert_eq!(rt.state.rebinding, Some(Action::Mute));
    assert_eq!(bindings(&rt, Action::Mute), vec![combo("M")]);
}

#[test]
fn leaving_settings_cancels_the_capture() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    rt.update(|ui, _| ui.close_settings());
    assert!(rt.state.rebinding.is_none());
}

#[test]
fn keys_are_not_captured_unless_asked() {
    let (mut rt, rx) = keybinds_runtime();
    press_key(&mut rt, Key::Function(10));
    press_char(&mut rt, "y");
    assert_eq!(rt.state.settings.keymap(), Settings::default().keymap());
    assert!(rx.try_recv().is_err());
}

#[test]
fn the_reset_button_restores_one_default() {
    let (mut rt, rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::SeekBack);
    press_char(&mut rt, "a");
    assert_eq!(bindings(&rt, Action::SeekBack), vec![combo("A")]);
    rx.try_iter().for_each(drop);
    tap(&mut rt, "bind-reset-seek_back");
    assert_eq!(
        bindings(&rt, Action::SeekBack),
        vec![combo("Left"), combo("J")]
    );
    assert!(rt.find("bind-reset-seek_back").is_none());
    assert!(rt.state.settings.keybinds.is_empty());
    assert!(last_applied(&rx).is_some());
}

#[test]
fn reset_all_restores_every_default_and_is_off_when_nothing_changed() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_char(&mut rt, "y");
    start_rebind(&mut rt, Action::SaveClip);
    press_key(&mut rt, Key::Backspace);
    tap(&mut rt, "bind-reset-all");
    assert_eq!(rt.state.settings.keymap(), Settings::default().keymap());
    assert!(rt.state.settings.keybinds.is_empty());
    let before = rt.state.settings.clone();
    tap(&mut rt, "bind-reset-all");
    assert_eq!(rt.state.settings, before);
}

#[test]
fn a_rebind_keeps_ids_this_version_does_not_know() {
    let (mut rt, _rx) = keybinds_runtime();
    rt.update(|ui, _| {
        ui.settings.keybinds.insert("teleport".into(), "T".into());
    });
    start_rebind(&mut rt, Action::Mute);
    press_char(&mut rt, "y");
    assert_eq!(
        rt.state
            .settings
            .keybinds
            .get("teleport")
            .map(String::as_str),
        Some("T")
    );
}

#[test]
fn a_clash_from_a_hand_edited_file_warns_on_both_rows() {
    let (mut rt, _rx) = keybinds_runtime();
    rt.update(|ui, _| {
        ui.settings.keybinds.insert("mute".into(), "F".into());
    });
    settle(&mut rt);
    assert!(has_text(&rt, "Also used by Toggle fullscreen."));
    assert!(has_text(&rt, "Also used by Mute."));
}

#[test]
fn a_failed_global_registration_shows_a_note_on_its_row_until_it_clears() {
    let (mut rt, _rx) = keybinds_runtime();
    let note = "F9 is already used by another app. Choose a different key.";
    rt.update(|ui, tx| {
        ui.apply(
            UiMsg::HotkeyStatus {
                errors: vec![(Action::SaveClip, note.into())],
            },
            tx,
        );
    });
    settle(&mut rt);
    assert!(has_text(&rt, note));
    rt.update(|ui, tx| ui.apply(UiMsg::HotkeyStatus { errors: Vec::new() }, tx));
    settle(&mut rt);
    assert!(!has_text(&rt, note));
    assert!(has_text(&rt, "Works in game"));
}

#[test]
fn a_rebound_review_key_works_and_the_old_one_does_not() {
    let mut rt = key_review_runtime();
    rt.update(|ui, _| {
        ui.settings.keybinds.insert("mute".into(), "Y".into());
    });
    press_char(&mut rt, "m");
    assert!(fake_player::calls().is_empty());
    press_char(&mut rt, "y");
    assert_eq!(fake_player::calls(), vec![Call::Muted(true)]);
}

#[test]
fn an_unbound_review_key_does_nothing() {
    let mut rt = key_review_runtime();
    rt.update(|ui, _| {
        ui.settings
            .keybinds
            .insert("play_pause".into(), "none".into());
    });
    press_char(&mut rt, " ");
    assert!(fake_player::calls().is_empty());
}

#[test]
fn rebinding_in_the_page_changes_what_the_review_keys_do() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::Mute);
    press_char(&mut rt, "y");
    rt.update(|ui, _| {
        ui.close_settings();
        ui.player_factory = fake_player::factory;
        ui.open_review_entry(crate::ui::review_entry());
    });
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    fake_player::clear_calls();
    press_char(&mut rt, "y");
    assert_eq!(fake_player::calls(), vec![Call::Muted(true)]);
}

#[test]
fn the_speed_keys_step_through_the_speeds_and_stop_at_the_ends() {
    let mut rt = key_review_runtime();
    let rate = |rt: &Runtime<Ui>| rt.state.review.as_ref().map(|r| r.speed_rate());
    press_char(&mut rt, "]");
    assert_eq!(rate(&rt), Some(1.25));
    press_char(&mut rt, "]");
    press_char(&mut rt, "]");
    assert_eq!(rate(&rt), Some(2.0));
    for _ in 0..7 {
        press_char(&mut rt, "[");
    }
    assert_eq!(rate(&rt), Some(0.25));
    let rates: Vec<f32> = fake_player::calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Rate(r) => Some(r),
            _ => None,
        })
        .collect();
    assert_eq!(rates, vec![1.25, 1.5, 2.0, 1.5, 1.25, 1.0, 0.75, 0.5, 0.25]);
}

#[test]
fn j_and_l_seek_like_the_arrows_and_shift_arrows_jump_between_events() {
    let mut rt = key_review_runtime();
    fake_player::set_time(6.0);
    press_char(&mut rt, "l");
    super::assert_seeks_to(10.0);
    fake_player::clear_calls();
    press_char(&mut rt, "j");
    super::assert_seeks_to(1.0);
    fake_player::clear_calls();
    fake_player::set_time(1.0);
    press_with(
        &mut rt,
        Key::Right,
        Mods {
            shift: true,
            ..Mods::default()
        },
    );
    super::assert_seeks_to(2.3);
    fake_player::clear_calls();
    fake_player::set_time(3.0);
    press_with(
        &mut rt,
        Key::Left,
        Mods {
            shift: true,
            ..Mods::default()
        },
    );
    super::assert_seeks_to(1.4);
    fake_player::set_time(0.0);
}

fn closed_settings_runtime() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (mut rt, rx) = keybinds_runtime();
    rt.update(|ui, _| ui.close_settings());
    rt.frame(1.0 / 60.0);
    (rt, rx)
}

pub(super) fn platform() -> Mods {
    if cfg!(target_os = "macos") {
        Mods {
            cmd: true,
            logo: true,
            ..Mods::default()
        }
    } else {
        ctrl()
    }
}

#[test]
fn a_command_key_press_goes_to_the_binding_with_the_primary_modifier() {
    let (mut rt, _rx) = closed_settings_runtime();
    let primary = Mods {
        cmd: true,
        ..Mods::default()
    };
    press_with(&mut rt, Key::Command(','), primary);
    assert!(rt.state.settings_open);
}

#[test]
fn the_platform_modifier_opens_settings_and_a_bare_key_does_not() {
    let (mut rt, _rx) = closed_settings_runtime();
    press_key(&mut rt, Key::Text(",".into()));
    assert!(!rt.state.settings_open);
    press_with(&mut rt, Key::Command(','), platform());
    assert!(rt.state.settings_open);
}

#[test]
fn rebinding_a_general_action_moves_the_shortcut() {
    let (mut rt, _rx) = keybinds_runtime();
    start_rebind(&mut rt, Action::OpenSettings);
    press_with(&mut rt, Key::Command('r'), ctrl());
    rt.update(|ui, _| ui.close_settings());
    press_with(&mut rt, Key::Command('r'), ctrl());
    assert!(rt.state.settings_open);
    rt.update(|ui, _| ui.close_settings());
    press_with(&mut rt, Key::Command(','), platform());
    assert!(!rt.state.settings_open);
}

#[test]
fn review_keys_do_nothing_without_a_game() {
    let (mut rt, _rx) = keybinds_runtime();
    press_char(&mut rt, "f");
    press_char(&mut rt, "m");
    assert!(rt.state.settings_open);
    assert!(rt.state.review.is_none());
}

#[test]
fn the_fixed_escape_key_still_leaves_fullscreen_and_closes_the_game() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    assert!(rt.state.in_theater());
    press_key(&mut rt, Key::Escape);
    assert!(!rt.state.in_theater());
    press_key(&mut rt, Key::Escape);
    assert_eq!(rt.state.detail(), Detail::Pick);
}
