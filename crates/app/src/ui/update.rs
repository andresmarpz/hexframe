use greyscreen_core::update::{UpdateInfo, UpdateState};
use greyscreen_engine::{EngineCmd, EngineStatus};

use super::Ui;

#[derive(Debug, Default)]
pub(crate) struct UpdateUi {
    pub state: UpdateState,
    pub install_when_idle: bool,
    pub dismissed_tag: Option<String>,
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

    pub(crate) fn update_banner(&self) -> Option<&UpdateInfo> {
        let UpdateState::Ready(info) = &self.update.state else {
            return None;
        };
        let hidden = self.update.dismissed_tag.as_deref() == Some(info.tag.as_str())
            || self.session_busy()
            || self.review.as_ref().is_some_and(|r| r.theater);
        (!hidden).then_some(info)
    }

    pub(crate) fn dismiss_update_banner(&mut self) {
        self.update.dismissed_tag = self.update_available.as_ref().map(|i| i.tag.clone());
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
