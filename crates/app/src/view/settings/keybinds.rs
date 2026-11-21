use greyscreen_core::keybinds::{Action, Keymap, Scope};
use rune::{El, Icon, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Keybind, Kit};

use super::setting_row;
use crate::ui::Ui;
use crate::ui::key_event::MAC;

const PRESS_KEYS: &str = "Press keys…";
const CAPTURE_HELP: &str = "Press the keys to use. Esc cancels, Backspace clears.";
const RESET_SLOT: f32 = 28.0;
const GLOBAL_HINT: &str = "Works in game";

static RESET: Icon = Icon {
    name: "greyscreen-reset",
    view: 16.0,
    body: r#"<path d="M3.2 8a4.8 4.8 0 1 0 1.5-3.5" stroke="currentColor" fill="none" stroke-linecap="round"/><path d="M3 2.6V5.4H5.8" stroke="currentColor" fill="none" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(super) fn header(s: &Ui, k: &Kit) -> El {
    let keymap = s.settings.keymap();
    let unchanged = Action::ALL.iter().all(|action| keymap.is_default(*action));
    let button = Button::new("Reset all to defaults")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .disabled(unchanged)
        .on_press(|ui: &mut Ui| ui.reset_all_bindings())
        .build(k)
        .key("bind-reset-all");
    row()
        .w_full()
        .center_y()
        .justify_between()
        .child(
            text("Click Change, then press the new keys.")
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(button)
}

pub(super) fn bind_row(s: &Ui, k: &Kit, action: Action) -> El {
    action_row(s, k, &s.settings.keymap(), action)
}

fn action_row(s: &Ui, k: &Kit, keymap: &Keymap, action: Action) -> El {
    let spec = action.spec();
    let capturing = s.rebinding == Some(action);
    let mut control = row().gap(10.0).center_y();
    if !capturing {
        control = control.child(chips(k, keymap, action));
    }
    if spec.fixed {
        control = control.child(
            text("Fixed")
                .font(k.ty.small)
                .color(k.c.text_faint)
                .key(format!("bind-fixed-{}", spec.id)),
        );
    } else {
        control = control.child(rebind_button(k, action, capturing));
        control = control.child(if keymap.is_default(action) {
            el().w(RESET_SLOT).h(RESET_SLOT).a11y_hidden()
        } else {
            reset_button(k, action)
        });
    }
    let note = note(s, keymap, action);
    setting_row(
        k,
        &format!("bind-{}", spec.id),
        spec.label,
        note.as_deref(),
        control,
    )
}

fn note(s: &Ui, keymap: &Keymap, action: Action) -> Option<String> {
    if let Some((_, hint)) = s.bind_hint.as_ref().filter(|(a, _)| *a == action) {
        return Some(hint.clone());
    }
    if s.rebinding == Some(action) {
        return Some(CAPTURE_HELP.to_owned());
    }
    if let Some((_, error)) = s.hotkey_errors.iter().find(|(a, _)| *a == action) {
        return Some(error.clone());
    }
    let others = keymap.conflicts(action);
    if !others.is_empty() {
        let names: Vec<&str> = others.iter().map(|other| other.label()).collect();
        return Some(format!("Also used by {}.", names.join(", ")));
    }
    (action.scope() == Scope::Global).then(|| GLOBAL_HINT.to_owned())
}

fn chips(k: &Kit, keymap: &Keymap, action: Action) -> El {
    let bindings = keymap.bindings(action);
    let id = action.id();
    if bindings.is_empty() {
        return text("Not set")
            .key(format!("bind-keys-{id}"))
            .font(k.ty.small)
            .color(k.c.text_faint);
    }
    let mut keys = row().key(format!("bind-keys-{id}")).gap(6.0).center_y();
    for (i, combo) in bindings.iter().enumerate() {
        if i > 0 {
            keys = keys.child(text("or").font(k.ty.small).color(k.c.text_faint));
        }
        keys = keys.child(Keybind::new(&combo.keycaps(MAC)).build(k));
    }
    keys
}

fn rebind_button(k: &Kit, action: Action, capturing: bool) -> El {
    let label = if capturing { PRESS_KEYS } else { "Change" };
    Button::new(label)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press_tx(move |ui: &mut Ui, tx: &mut rune::Tx| ui.toggle_rebind(action, tx))
        .build(k)
        .key(format!("bind-btn-{}", action.id()))
}

fn reset_button(k: &Kit, action: Action) -> El {
    IconButton::new(&RESET)
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .label(format!("Reset {} to default", action.label()))
        .help("Reset to default")
        .on_press(move |ui: &mut Ui| ui.reset_binding(action))
        .build(k)
        .key(format!("bind-reset-{}", action.id()))
}
