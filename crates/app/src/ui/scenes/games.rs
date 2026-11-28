use rune::{PointerPress, TextEdit};

use super::{Scene, first_run, game, idle};
use crate::ui::Screen;

pub(super) const SCENES: &[Scene] = &[
    ("games-wide", |ui| {
        game(ui);
        ui.settings.list_width = 440;
    }),
    ("games-narrow", |ui| {
        game(ui);
        ui.settings.list_width = 240;
    }),
    ("games-pick", idle),
    ("clips", |ui| {
        idle(ui);
        ui.go(Screen::Clips);
    }),
    ("clips-empty", |ui| {
        first_run(ui);
        ui.go(Screen::Clips);
    }),
    ("recordings", game),
    ("recordings-empty", first_run),
    ("recordings-filtered", |ui| {
        idle(ui);
        ui.filter.champion = TextEdit::new("ahri");
    }),
    ("recordings-no-match", |ui| {
        idle(ui);
        ui.filter.champion = TextEdit::new("zzz");
    }),
    ("recordings-delete", |ui| {
        game(ui);
        ui.filter.pending_delete = ui.review.as_ref().map(|r| r.entry.meta.id.clone());
    }),
    ("row-menu", |ui| {
        idle(ui);
        if let Some(id) = ui.recordings.first().map(|rec| rec.meta.id.clone()) {
            ui.open_row_menu(id, PointerPress { x: 140.0, y: 190.0 });
        }
    }),
];
