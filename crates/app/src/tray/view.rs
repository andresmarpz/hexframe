use greyscreen_core::problem::Problem;
use greyscreen_core::time::{Hns, hns_to_secs};
use greyscreen_engine::EngineStatus;
use rune::TrayItem;

use crate::ui::format_clock;

pub(super) const OPEN_ID: &str = "open";
pub(super) const TOGGLE_ID: &str = "toggle";
pub(super) const SAVE_ID: &str = "save";
pub(super) const QUIT_ID: &str = "quit";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrayState {
    pub status: EngineStatus,
    pub auto_record: bool,
}

impl TrayState {
    pub(crate) fn is_recording(&self) -> bool {
        matches!(self.status, EngineStatus::Recording { .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct View {
    pub tooltip: String,
    pub recording: bool,
    pub items: Vec<TrayItem>,
}

impl View {
    pub(super) fn of(state: &TrayState, now: Hns) -> View {
        View {
            tooltip: format!("Greyscreen — {}", status_line(state, now)),
            recording: state.is_recording(),
            items: items(state),
        }
    }
}

fn status_line(state: &TrayState, now: Hns) -> String {
    match &state.status {
        EngineStatus::Recording { since, .. } => {
            let secs = hns_to_secs((now - since).max(0)) as u32;
            format!("Recording {}", format_clock(secs))
        }
        EngineStatus::Idle if state.auto_record => "Waiting for League".to_owned(),
        EngineStatus::Idle | EngineStatus::GameRunning => "Recording paused".to_owned(),
        EngineStatus::Waiting => "League is loading".to_owned(),
        EngineStatus::Finalizing => "Saving".to_owned(),
        EngineStatus::Error(raw) => Problem::from_error(raw).title().to_owned(),
    }
}

fn items(state: &TrayState) -> Vec<TrayItem> {
    let toggle = if state.auto_record {
        "Pause recording"
    } else {
        "Resume recording"
    };
    vec![
        TrayItem::action(OPEN_ID, "Open Greyscreen"),
        TrayItem::Separator,
        TrayItem::action(TOGGLE_ID, toggle),
        TrayItem::action(SAVE_ID, "Save clip").enabled(state.is_recording()),
        TrayItem::Separator,
        TrayItem::action(QUIT_ID, "Quit"),
    ]
}
