use greyscreen_core::keybinds::Action;
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Checkbox, ControlSize, Keybind, Kit, TextInput};

use super::catalog::Row;
use super::{setting_row, stacked_row, toggle_row};
use crate::ui::MAC;
use crate::ui::{SettingsSection, Ui, parse_clip_secs};

const MOMENTS_NOTE: &str =
    "Objectives are Dragon, Herald, Baron and Voidgrubs. Structures are turrets and inhibitors.";
const WINDOW_NOTE: &str = "Seconds kept around each moment. Manual clips use the same window.";
const HOTKEY_NOTE: &str = "Saves a clip of the last moments, even in game.";
const INPUT_WIDTH: f32 = 72.0;
const COLUMNS: usize = 3;

type Moment = (&'static str, &'static str, bool, fn(&mut Ui));

pub(super) fn render(s: &Ui, k: &Kit, which: Row) -> El {
    match which {
        Row::ClipsEnabled => toggle_row(
            k,
            "clips-enabled",
            "Save highlight clips",
            Some("Cut short clips around the moments you pick."),
            s.settings.clips.enabled,
            |ui| ui.change(|s| s.clips.enabled = !s.clips.enabled),
        ),
        Row::Moments => stacked_row(
            k,
            "moments-row",
            "Moments",
            Some(MOMENTS_NOTE),
            moments(s, k),
        ),
        Row::ClipWindow => setting_row(
            k,
            "window-row",
            "Clip window",
            Some(WINDOW_NOTE),
            window(s, k),
        ),
        _ => setting_row(
            k,
            "hotkey-row",
            "Save clip hotkey",
            Some(HOTKEY_NOTE),
            hotkey(s, k),
        ),
    }
}

fn moments(s: &Ui, k: &Kit) -> El {
    let clips = &s.settings.clips;
    let on = clips.enabled;
    let items: [Moment; 7] = [
        ("clip-kills", "Kills", clips.kills, |ui| {
            ui.change(|s| s.clips.kills = !s.clips.kills)
        }),
        ("clip-deaths", "Deaths", clips.deaths, |ui| {
            ui.change(|s| s.clips.deaths = !s.clips.deaths)
        }),
        ("clip-assists", "Assists", clips.assists, |ui| {
            ui.change(|s| s.clips.assists = !s.clips.assists)
        }),
        ("clip-multikills", "Multikills", clips.multikills, |ui| {
            ui.change(|s| s.clips.multikills = !s.clips.multikills)
        }),
        ("clip-objectives", "Objectives", clips.objectives, |ui| {
            ui.change(|s| s.clips.objectives = !s.clips.objectives)
        }),
        ("clip-structures", "Structures", clips.structures, |ui| {
            ui.change(|s| s.clips.structures = !s.clips.structures)
        }),
        ("clip-aces", "Aces", clips.aces, |ui| {
            ui.change(|s| s.clips.aces = !s.clips.aces)
        }),
    ];
    let mut grid = col().key("moments").w_full().gap(6.0);
    for line in items.chunks(COLUMNS) {
        let mut cells = row().w_full();
        for (key, label, checked, flip) in line {
            let cell = Checkbox::new(*checked)
                .label(*label)
                .disabled(!on)
                .on_toggle(*flip)
                .build(k)
                .key(*key);
            cells = cells.child(col().w_frac(1.0 / COLUMNS as f32).child(cell));
        }
        grid = grid.child(cells);
    }
    grid
}

fn window(s: &Ui, k: &Kit) -> El {
    row()
        .gap(10.0)
        .center_y()
        .child(secs(k, "pre-secs", "Before", &s.pre_secs, |ui| {
            &mut ui.pre_secs
        }))
        .child(secs(k, "post-secs", "After", &s.post_secs, |ui| {
            &mut ui.post_secs
        }))
}

fn secs(
    k: &Kit,
    key: &'static str,
    label: &str,
    edit: &rune::TextEdit,
    pick: for<'a> fn(&'a mut Ui) -> &'a mut rune::TextEdit,
) -> El {
    let invalid = parse_clip_secs(&edit.text).is_none();
    let input = TextInput::new(edit)
        .label(format!("Seconds {}", label.to_lowercase()))
        .width(Some(INPUT_WIDTH))
        .invalid(invalid)
        .on_edit(pick)
        .on_change(|ui: &mut Ui, _: &mut rune::Tx| ui.edit_clip_secs())
        .build(k)
        .key(key);
    row()
        .gap(6.0)
        .center_y()
        .child(
            text(label.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(input)
        .child(text("s").font(k.ty.small).color(k.c.text_muted))
}

fn hotkey(s: &Ui, k: &Kit) -> El {
    let keymap = s.settings.keymap();
    let mut keys = row().key("hotkey-keys").gap(6.0).center_y();
    for combo in keymap.bindings(Action::SaveClip) {
        keys = keys.child(Keybind::new(&combo.keycaps(MAC)).build(k));
    }
    let change = Button::new("Change")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.pick_settings_section(SettingsSection::Keybinds))
        .build(k)
        .key("hotkey-change");
    row().gap(10.0).center_y().child(keys).child(change)
}
