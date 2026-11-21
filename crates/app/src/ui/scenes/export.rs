use std::path::PathBuf;

use greyscreen_core::export::{ExportDone, ExportPreset, ExportRange};

use super::{Scene, game};
use crate::ui::{CopyState, ExportPhase, Ui};

fn clip(ui: &mut Ui) {
    game(ui);
    let id = ui
        .review
        .as_ref()
        .map(|review| review.entry.meta.id.clone())
        .unwrap_or_default();
    ui.open_export_clip(&id, 1);
}

fn discord(ui: &mut Ui) {
    clip(ui);
    ui.export.can_reencode = true;
    ui.export.preset = ExportPreset::Discord;
}

fn working(ui: &mut Ui, fraction: f32, shrinking: bool, cancelling: bool) {
    discord(ui);
    ui.export.phase = ExportPhase::Working {
        fraction,
        shrinking,
        cancelling,
    };
}

fn done(ui: &mut Ui, preset: ExportPreset, bytes: u64, shrunk: bool, copy: CopyState) {
    clip(ui);
    let wanted_start = ui.export.range.start;
    let range = match preset {
        ExportPreset::Original => ExportRange::new(wanted_start - 0.6, ui.export.range.end),
        ExportPreset::Discord => ui.export.range,
    };
    ui.export.phase = ExportPhase::Done {
        done: ExportDone {
            path: PathBuf::from("exports/Ahri_00-01_00-06_Original.mp4"),
            bytes,
            range,
            preset,
            shrunk,
        },
        wanted_start,
        copy,
    };
}

pub(super) const SCENES: &[Scene] = &[
    ("export-clip", clip),
    ("export-moment", |ui| {
        game(ui);
        ui.open_export_moment();
    }),
    ("export-discord", discord),
    ("export-discord-long", |ui| {
        discord(ui);
        ui.export.range = ExportRange::new(0.0, 300.0);
        ui.export.default_range = ui.export.range;
        ui.export.bounds = (0.0, 300.0);
    }),
    ("export-windows-only", |ui| {
        clip(ui);
        ui.export.can_reencode = false;
    }),
    ("export-working", |ui| working(ui, 0.42, false, false)),
    ("export-shrinking", |ui| working(ui, 0.15, true, false)),
    ("export-cancelling", |ui| working(ui, 0.6, false, true)),
    ("export-done", |ui| {
        done(
            ui,
            ExportPreset::Original,
            18_400_000,
            false,
            CopyState::Idle,
        );
    }),
    ("export-done-discord", |ui| {
        done(ui, ExportPreset::Discord, 8_700_000, true, CopyState::Idle);
    }),
    ("export-copied", |ui| {
        done(
            ui,
            ExportPreset::Discord,
            8_700_000,
            false,
            CopyState::Copied,
        );
    }),
    ("export-error", |ui| {
        discord(ui);
        ui.export.phase =
            ExportPhase::Failed("Couldn't make the smaller file. Try Original quality.".into());
    }),
];
