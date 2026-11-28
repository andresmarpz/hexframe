use super::{Scene, idle};
use crate::ui::{Screen, Ui, preview};

fn open(ui: &mut Ui, index: usize) {
    idle(ui);
    if let Some(id) = ui.recordings.get(index).map(|rec| rec.meta.id.clone()) {
        ui.open_game(id);
    }
}

fn sized(ui: &mut Ui, minutes: f64, game_page: bool) {
    idle(ui);
    if let Some(rec) = ui.recordings.first_mut() {
        preview::set_minutes(rec, minutes);
    }
    if game_page {
        open(ui, 0);
    } else {
        ui.go(Screen::Home);
    }
}

pub(super) const SCENES: &[Scene] = &[
    ("game-page", |ui| open(ui, 0)),
    ("game-page-defeat-no-clips", |ui| open(ui, 4)),
    ("timeline-short", |ui| sized(ui, 6.0, false)),
    ("timeline-long", |ui| sized(ui, 48.0, false)),
    ("timeline-short-game", |ui| sized(ui, 6.0, true)),
    ("timeline-long-game", |ui| sized(ui, 48.0, true)),
    ("timeline-huge", |ui| sized(ui, 90.0, false)),
];
