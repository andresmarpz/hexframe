use crate::qa::reveal;
use greyscreen_core::paths;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{El, Tx, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::ui::{TEST_RECORDING_SECS, Ui};

pub(super) fn bar(s: &Ui, k: &Kit) -> El {
    match &s.status {
        EngineStatus::Recording { .. } => recording(s, k),
        EngineStatus::Error(_) => failed(k),
        status => idle(status, k),
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

fn failed(k: &Kit) -> El {
    row()
        .gap(10.0)
        .center_y()
        .child(primary(k, "Retry", "retry-capture", |ui, _| {
            ui.retry_capture()
        }))
        .child(secondary(
            k,
            "Open logs folder",
            "error-open-logs",
            false,
            |_, _| reveal(&paths::logs_dir()),
        ))
}

fn idle(status: &EngineStatus, k: &Kit) -> El {
    let reason = match status {
        EngineStatus::Idle | EngineStatus::GameRunning => None,
        EngineStatus::Finalizing => Some("Not available while the last game is saving."),
        _ => Some("Not available while League is starting."),
    };
    let mut bar = row().gap(12.0).center_y().child(secondary(
        k,
        "Test recording (10 s)",
        "test-recording",
        reason.is_some(),
        |ui, _| {
            ui.send(EngineCmd::StartTestRecording {
                secs: TEST_RECORDING_SECS,
            })
        },
    ));
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
    let ink = k.c.text_accent;
    Button::new(label)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .label_with(move |font, _| text(label).font(font).color(ink))
        .on_press_tx(on_press)
        .build(k)
        .key(key)
        .fill(k.alpha(ink, 0.16))
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
    let label = if confirming {
        "Click again to stop"
    } else {
        "Stop"
    };
    let (rest, hover) = (k.c.text_muted, k.c.danger.fg);
    let ink = if confirming { hover } else { rest };
    Button::new(label)
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Large)
        .label_with(move |font, _| text(label).font(font).color(ink).group_color(hover))
        .on_press_tx(|ui: &mut Ui, tx| ui.request_stop(tx))
        .build(k)
        .key("stop-recording")
        .hover_group()
}
