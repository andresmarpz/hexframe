use greyscreen_core::problem::{FOLDER_UNUSABLE, NO_FRAMES};
use greyscreen_engine::EngineStatus;

use super::{Scene, home};
use crate::ui::Ui;

fn failed(ui: &mut Ui, raw: &str) {
    home(ui);
    ui.status = EngineStatus::Error(raw.to_owned());
}

pub(super) const SCENES: &[Scene] = &[
    ("error-fullscreen", |ui| failed(ui, NO_FRAMES)),
    ("error-league", |ui| {
        failed(ui, "the captured window closed during startup");
    }),
    ("error-folder", |ui| {
        failed(
            ui,
            &format!("{FOLDER_UNUSABLE}: D:\\Videos\\Greyscreen: Access is denied. (os error 5)"),
        );
    }),
    ("error-disk", |ui| {
        failed(
            ui,
            &format!(
                "{FOLDER_UNUSABLE}: D:\\Videos\\Greyscreen: There is not enough space on the disk. (os error 112)"
            ),
        );
    }),
    ("error-encoder", |ui| {
        failed(
            ui,
            "video encoder failed: no usable hardware H.264 encoder: no MFT",
        );
    }),
    ("error-windows", |ui| {
        failed(
            ui,
            "screen capture failed to start: windows graphics capture is not supported",
        );
    }),
    ("error-blocked", |ui| {
        failed(
            ui,
            "screen capture failed to start: windows api error: Access is denied. (0x80070005)",
        );
    }),
];
