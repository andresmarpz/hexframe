use greyscreen_core::update::{UpdateInfo, UpdateState};
use rune::El;
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::{setting_row, toggle_row};
use crate::ui::Ui;
use crate::view::widgets::busy_label_content;

const CHECK_WIDTH: f32 = 92.0;
const INSTALL_WIDTH: f32 = 156.0;
const OPEN_RELEASE_WIDTH: f32 = 120.0;

pub(super) fn auto_row(s: &Ui, k: &Kit) -> El {
    toggle_row(
        k,
        "check-updates",
        "Check for updates automatically",
        Some("Look for a newer version every few hours."),
        s.settings.check_updates,
        |ui| ui.change(|s| s.check_updates = !s.check_updates),
    )
}

pub(super) fn update_row(s: &Ui, k: &Kit) -> El {
    let (description, action) = describe(s);
    setting_row(
        k,
        "update-row",
        "Updates",
        Some(&description),
        action.build(k),
    )
}

enum Action {
    Check { enabled: bool, busy: bool },
    Install { enabled: bool },
    OpenRelease(String),
}

impl Action {
    fn build(self, k: &Kit) -> El {
        match self {
            Action::Check { enabled, busy } => check_button(k, enabled, busy),
            Action::Install { enabled } => Button::new("Install and restart")
                .variant(ButtonVariant::Contrast)
                .control_size(ControlSize::Large)
                .disabled(!enabled)
                .on_press(Ui::install_update)
                .build(k)
                .key("install-update")
                .w(INSTALL_WIDTH),
            Action::OpenRelease(url) => Button::new("Open GitHub")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .on_press_tx(move |_: &mut Ui, tx: &mut rune::Tx| {
                    if let Err(err) = tx.open_url(&url) {
                        tracing::warn!(%err, "cannot open the release page");
                    }
                })
                .build(k)
                .key("open-release")
                .w(OPEN_RELEASE_WIDTH),
        }
    }
}

fn check_button(k: &Kit, enabled: bool, busy: bool) -> El {
    let kit = k.clone();
    Button::new("Check")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .disabled(!enabled && !busy)
        .label_with(move |font, ink| {
            busy_label_content(&kit, "Check", busy, "check-updates-spinner", font, ink)
        })
        .on_press(move |ui: &mut Ui| {
            if !busy {
                ui.check_for_updates();
            }
        })
        .build(k)
        .key("check-updates-button")
        .w(CHECK_WIDTH)
}

fn check(enabled: bool) -> Action {
    Action::Check {
        enabled,
        busy: false,
    }
}

fn working() -> Action {
    Action::Check {
        enabled: false,
        busy: true,
    }
}

fn describe(s: &Ui) -> (String, Action) {
    let available = |info: &UpdateInfo| format!("Update to v{} available", info.version());
    match &s.update.state {
        UpdateState::Unknown => ("See if a newer version is out.".into(), check(true)),
        UpdateState::Checking => ("Checking…".into(), working()),
        UpdateState::UpToDate => ("Greyscreen is up to date.".into(), check(true)),
        UpdateState::AvailableManual(info) => (
            format!("{} — download from GitHub", available(info)),
            Action::OpenRelease(info.notes_url.clone()),
        ),
        UpdateState::Available(info) => (
            format!("{}. It downloads after this game.", available(info)),
            check(false),
        ),
        UpdateState::Downloading(info) => (format!("Downloading v{}…", info.version()), working()),
        UpdateState::Ready(_) if s.update.install_when_idle => (
            "Greyscreen will update after this game".into(),
            Action::Install { enabled: false },
        ),
        UpdateState::Ready(info) => (
            format!("{}. Greyscreen restarts to finish.", available(info)),
            Action::Install { enabled: true },
        ),
        UpdateState::Installing(_) => (
            "Restarting to finish the update…".into(),
            Action::Install { enabled: false },
        ),
        UpdateState::Failed(text) => (text.clone(), check(true)),
    }
}
