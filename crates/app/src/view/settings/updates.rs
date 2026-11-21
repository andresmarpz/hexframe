use greyscreen_core::update::{UpdateInfo, UpdateState};
use rune::El;
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::{setting_row, toggle_row};
use crate::ui::Ui;

pub(super) fn rows(s: &Ui, k: &Kit) -> Vec<El> {
    vec![
        update_row(s, k),
        toggle_row(
            k,
            "check-updates",
            "Check for updates automatically",
            Some("Look for a newer version every few hours."),
            s.settings.check_updates,
            |ui| ui.change(|s| s.check_updates = !s.check_updates),
        ),
    ]
}

fn update_row(s: &Ui, k: &Kit) -> El {
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
    Check { enabled: bool, label: &'static str },
    Install { enabled: bool },
    OpenRelease(String),
}

impl Action {
    fn build(self, k: &Kit) -> El {
        match self {
            Action::Check { enabled, label } => Button::new(label)
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .disabled(!enabled)
                .on_press(Ui::check_for_updates)
                .build(k)
                .key("check-updates-button"),
            Action::Install { enabled } => Button::new("Install and restart")
                .variant(ButtonVariant::Contrast)
                .control_size(ControlSize::Large)
                .disabled(!enabled)
                .on_press(Ui::install_update)
                .build(k)
                .key("install-update"),
            Action::OpenRelease(url) => Button::new("Open GitHub")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .on_press_tx(move |_: &mut Ui, tx: &mut rune::Tx| {
                    if let Err(err) = tx.open_url(&url) {
                        tracing::warn!(%err, "cannot open the release page");
                    }
                })
                .build(k)
                .key("open-release"),
        }
    }
}

fn check(enabled: bool, label: &'static str) -> Action {
    Action::Check { enabled, label }
}

fn describe(s: &Ui) -> (String, Action) {
    let available = |info: &UpdateInfo| format!("Update to v{} available", info.version());
    match &s.update.state {
        UpdateState::Unknown => (
            "See if a newer version is out.".into(),
            check(true, "Check for updates"),
        ),
        UpdateState::Checking => ("Checking…".into(), check(false, "Checking…")),
        UpdateState::UpToDate => (
            "Greyscreen is up to date.".into(),
            check(true, "Check for updates"),
        ),
        UpdateState::AvailableManual(info) => (
            format!("{} — download from GitHub", available(info)),
            Action::OpenRelease(info.notes_url.clone()),
        ),
        UpdateState::Available(info) => (
            format!("{}. It downloads after this game.", available(info)),
            check(false, "Check for updates"),
        ),
        UpdateState::Downloading(info) => (
            format!("Downloading v{}…", info.version()),
            check(false, "Downloading…"),
        ),
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
        UpdateState::Failed(text) => (text.clone(), check(true, "Check for updates")),
    }
}
