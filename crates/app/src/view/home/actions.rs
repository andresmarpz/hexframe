use crate::headless::reveal;
use greyscreen_core::paths;
use greyscreen_core::problem::Problem;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{El, Tx, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::ui::{TEST_RECORDING_SECS, Ui};
use crate::view::widgets::stable_label_content;

pub(super) fn bar(s: &Ui, k: &Kit) -> El {
    match &s.status {
        EngineStatus::Recording { .. } => recording(s, k),
        EngineStatus::Error(message) => failed(Problem::from_error(message), k),
        _ if !s.settings.first_run_done => row().key("first-run-quiet"),
        status => idle(status, !s.recordings.is_empty(), k),
    }
}

fn recording(s: &Ui, k: &Kit) -> El {
    row()
        .gap(10.0)
        .center_y()
        .child(primary(k, "Save clip (last 30 s)", "save-clip", |ui, _| {
            ui.send(EngineCmd::SaveClipNow)
        }))
        .child(stop_button(s.stop_confirm_until.is_some(), k))
}

fn failed(problem: Problem, k: &Kit) -> El {
    let mut bar = row().gap(10.0).center_y();
    if problem.offers_folder() {
        bar = bar.child(primary(
            k,
            "Choose folder…",
            "error-choose-folder",
            |ui, tx| ui.choose_folder(tx),
        ));
    }
    bar = if problem.offers_folder() {
        bar.child(secondary(k, "Retry", "retry-capture", false, |ui, _| {
            ui.retry_capture()
        }))
    } else {
        bar.child(primary(k, "Retry", "retry-capture", |ui, _| {
            ui.retry_capture()
        }))
    };
    let url = problem.help_url();
    bar = bar.child(
        Button::new("Learn more")
            .variant(ButtonVariant::Ghost)
            .control_size(ControlSize::Large)
            .on_press_tx(move |_: &mut Ui, tx: &mut Tx| {
                if let Err(err) = tx.open_url(&url) {
                    tracing::warn!(%err, "cannot open the help page");
                }
            })
            .build(k)
            .key("error-learn-more"),
    );
    if problem == Problem::Other {
        bar = bar.child(secondary(
            k,
            "Open logs folder",
            "error-open-logs",
            false,
            |_, _| reveal(&paths::logs_dir()),
        ));
    }
    bar
}

fn idle(status: &EngineStatus, has_games: bool, k: &Kit) -> El {
    let reason = match status {
        EngineStatus::Idle | EngineStatus::GameRunning => None,
        EngineStatus::Finalizing => Some("Not available while the last game is saving."),
        _ => Some("Not available while League is starting."),
    };
    let variant = if has_games {
        ButtonVariant::Ghost
    } else {
        ButtonVariant::Outline
    };
    let test = Button::new("Test recording (10 s)")
        .variant(variant)
        .control_size(ControlSize::Large)
        .disabled(reason.is_some())
        .on_press_tx(|ui: &mut Ui, _| {
            ui.send(EngineCmd::StartTestRecording {
                secs: TEST_RECORDING_SECS,
            })
        })
        .build(k)
        .key("test-recording");
    let mut bar = row().gap(12.0).center_y().child(test);
    if let Some(reason) = reason {
        bar = bar.child(
            text(reason)
                .key("test-recording-reason")
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    }
    bar
}

fn primary(
    k: &Kit,
    label: &'static str,
    key: &'static str,
    on_press: impl Fn(&mut Ui, &mut Tx) + 'static,
) -> El {
    Button::new(label)
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large)
        .on_press_tx(on_press)
        .build(k)
        .key(key)
}

fn secondary(
    k: &Kit,
    label: &'static str,
    key: &'static str,
    disabled: bool,
    on_press: impl Fn(&mut Ui, &mut Tx) + 'static,
) -> El {
    Button::new(label)
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Large)
        .disabled(disabled)
        .on_press_tx(on_press)
        .build(k)
        .key(key)
}

fn stop_button(confirming: bool, k: &Kit) -> El {
    let label = if confirming { "Confirm?" } else { "Stop" };
    let variant = if confirming {
        ButtonVariant::Danger
    } else {
        ButtonVariant::Outline
    };
    Button::new(label)
        .variant(variant)
        .control_size(ControlSize::Large)
        .label_with(move |font, ink| stable_label_content("Confirm?", label, font, ink))
        .on_press_tx(|ui: &mut Ui, tx| ui.request_stop(tx))
        .build(k)
        .key("stop-recording")
}
