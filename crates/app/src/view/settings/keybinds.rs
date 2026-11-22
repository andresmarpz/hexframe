use greyscreen_core::keybinds::{Action, Keymap, Scope};
use rune::{El, Icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Keybind, Kit};

use super::setting_row;
use crate::ui::MAC;
use crate::ui::Ui;

const PRESS_KEYS: &str = "Press keys…";
const CAPTURE_HELP: &str = "Press the keys to use. Esc cancels, Backspace clears.";
const IDLE_HELP: &str = "Click any shortcut, then press the new keys.";
const GLOBAL_HINT: &str = "Works in game";
const PILL_MIN_WIDTH: f32 = 108.0;

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
    let (hint, color) = if s.rebinding.is_some() {
        (CAPTURE_HELP, k.c.text_base)
    } else {
        (IDLE_HELP, k.c.text_muted)
    };
    row()
        .w_full()
        .center_y()
        .justify_between()
        .child(text(hint).font(k.ty.small).color(color))
        .child(button)
}

pub(super) fn bind_row(s: &Ui, k: &Kit, action: Action) -> El {
    action_row(s, k, &s.settings.keymap(), action)
}

fn action_row(s: &Ui, k: &Kit, keymap: &Keymap, action: Action) -> El {
    let spec = action.spec();
    let capturing = s.rebinding == Some(action);
    let mut control = row().gap(8.0).center_y();
    if spec.fixed {
        control = control.child(chips(k, keymap, action)).child(
            text("Fixed")
                .font(k.ty.small)
                .color(k.c.text_faint)
                .key(format!("bind-fixed-{}", spec.id)),
        );
    } else {
        if !keymap.is_default(action) {
            control = control.child(reset_button(k, action));
        }
        control = control.child(rebind_button(k, keymap, action, capturing));
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

fn binding_label(keymap: &Keymap, action: Action) -> String {
    let bindings = keymap.bindings(action);
    if bindings.is_empty() {
        return "Not set".to_owned();
    }
    bindings
        .iter()
        .map(|combo| combo.keycaps(MAC).join(" "))
        .collect::<Vec<_>>()
        .join(" or ")
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

fn rebind_button(k: &Kit, keymap: &Keymap, action: Action, capturing: bool) -> El {
    let (variant, label) = if capturing {
        (ButtonVariant::Outline, PRESS_KEYS.to_owned())
    } else {
        (ButtonVariant::Neutral, binding_label(keymap, action))
    };
    let mut button = Button::new(label)
        .variant(variant)
        .control_size(ControlSize::Regular)
        .on_press_tx(move |ui: &mut Ui, tx: &mut rune::Tx| ui.toggle_rebind(action, tx));
    if !capturing {
        let body = chips(k, keymap, action);
        button = button.label_with(move |_, _| body);
    }
    button
        .build(k)
        .min_w(PILL_MIN_WIDTH)
        .key(format!("bind-btn-{}", action.id()))
}

fn reset_button(k: &Kit, action: Action) -> El {
    IconButton::new(&RESET)
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .label(format!("Reset {} to default", action.label()))
        .help("Reset to default")
        .on_press(move |ui: &mut Ui| ui.reset_binding(action))
        .build(k)
        .key(format!("bind-reset-{}", action.id()))
}
