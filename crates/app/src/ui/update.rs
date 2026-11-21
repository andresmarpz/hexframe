use greyscreen_core::update::UpdateState;
use greyscreen_engine::{EngineCmd, EngineStatus};

use super::Ui;

#[derive(Debug, Default)]
pub(crate) struct UpdateUi {
    pub state: UpdateState,
    pub install_when_idle: bool,
}

impl Ui {
    pub(crate) fn session_busy(&self) -> bool {
        matches!(
            self.status,
            EngineStatus::Recording { .. } | EngineStatus::Finalizing | EngineStatus::Waiting
        )
    }

    pub(crate) fn check_for_updates(&mut self) {
        if matches!(
            self.update.state,
            UpdateState::Checking | UpdateState::Downloading(_) | UpdateState::Installing(_)
        ) {
            return;
        }
        self.apply_update(UpdateState::Checking);
        self.send(EngineCmd::CheckForUpdates);
    }

    pub(crate) fn install_update(&mut self) {
        if !matches!(self.update.state, UpdateState::Ready(_)) {
            return;
        }
        if self.session_busy() {
            self.update.install_when_idle = true;
        } else {
            self.send(EngineCmd::InstallUpdate);
        }
    }

    pub(crate) fn apply_update(&mut self, state: UpdateState) {
        self.update_available = state.info().cloned();
        if !matches!(state, UpdateState::Ready(_)) {
            self.update.install_when_idle = false;
        }
        self.update.state = state;
        self.install_if_idle();
    }

    pub(super) fn install_if_idle(&mut self) {
        if self.update.install_when_idle
            && !self.session_busy()
            && matches!(self.update.state, UpdateState::Ready(_))
        {
            self.send(EngineCmd::InstallUpdate);
        }
    }
}

pub(super) fn preview_info() -> greyscreen_core::update::UpdateInfo {
    greyscreen_core::update::UpdateInfo {
        tag: "v0.2.0".into(),
        prerelease: false,
        size: 24_000_000,
        notes_url: greyscreen_core::update::notes_url("v0.2.0"),
    }
}
