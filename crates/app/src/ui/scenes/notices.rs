use std::path::PathBuf;

use greyscreen_engine::{LOW_DISK_URL, NoticeAction, NoticeLevel};

use super::{Scene, home};
use crate::ui::{Notice, Ui};

fn notice(ui: &mut Ui, level: NoticeLevel, text: &str, action: Option<NoticeAction>) {
    home(ui);
    let notice = Notice {
        level,
        text: text.to_owned(),
        action,
    };
    ui.notice = Some((notice, f64::INFINITY));
}

pub(super) const SCENES: &[Scene] = &[
    ("notice-recovered", |ui| {
        notice(
            ui,
            NoticeLevel::Info,
            "Recovered 1 recording that was cut off.",
            None,
        );
    }),
    ("notice-recover-failed", |ui| {
        notice(
            ui,
            NoticeLevel::Warn,
            "Couldn't recover 2026-11-21_14-30-00_Ahri_CLASSIC",
            Some(NoticeAction::Reveal(PathBuf::from("recordings"))),
        );
    }),
    ("notice-low-disk", |ui| {
        notice(
            ui,
            NoticeLevel::Error,
            "Not enough disk space to record (1.4 GB free). Free up space or change the recordings folder.",
            Some(NoticeAction::Link(LOW_DISK_URL.to_owned())),
        );
    }),
    ("notice-disk-stopped", |ui| {
        notice(
            ui,
            NoticeLevel::Error,
            "Recording stopped: the disk is almost full (0.3 GB free). Free up space or change the recordings folder.",
            Some(NoticeAction::Link(LOW_DISK_URL.to_owned())),
        );
    }),
];
