use super::{Scene, idle};
use crate::ui::Ui;

fn open(ui: &mut Ui, index: usize) {
    idle(ui);
    if let Some(id) = ui.recordings.get(index).map(|rec| rec.meta.id.clone()) {
        ui.open_game(id);
    }
}

pub(super) const SCENES: &[Scene] = &[
    ("game-page", |ui| open(ui, 0)),
    ("game-page-defeat-no-clips", |ui| open(ui, 4)),
];
