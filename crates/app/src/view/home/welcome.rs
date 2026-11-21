use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::ui::{TEST_RECORDING_SECS, Ui};
use crate::view::card;

pub(super) fn card_for(s: &Ui, k: &Kit) -> El {
    let testable = matches!(s.status, EngineStatus::Idle | EngineStatus::GameRunning);
    let test = Button::new("Run test recording")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Large)
        .disabled(!testable)
        .on_press(|ui: &mut Ui| {
            ui.send(EngineCmd::StartTestRecording {
                secs: TEST_RECORDING_SECS,
            })
        })
        .build(k)
        .key("welcome-test-recording");
    card(k)
        .key("welcome")
        .padding(20.0)
        .gap(16.0)
        .child(
            col()
                .gap(2.0)
                .child(
                    text("Welcome to Greyscreen")
                        .font(k.ty.body_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text("Three steps and your first game is saved.")
                        .font(k.ty.body)
                        .color(k.c.text_muted),
                ),
        )
        .child(step(
            k,
            1,
            "Set League to Borderless or Windowed",
            "Greyscreen can't record the game in Fullscreen.",
            None,
        ))
        .child(step(
            k,
            2,
            "Run a 10-second test recording",
            "Check that video and sound are saved before your first game.",
            Some(test),
        ))
        .child(step(
            k,
            3,
            "Leave Record games on and play",
            "Greyscreen starts and stops by itself with each game.",
            None,
        ))
}

fn step(k: &Kit, number: u32, title: &str, detail: &str, action: Option<El>) -> El {
    let mut line = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(
            row()
                .size(24.0, 24.0)
                .radius(12.0)
                .fill(k.c.layer_02)
                .center()
                .a11y_hidden()
                .child(
                    text(number.to_string())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_muted),
                ),
        )
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(title.to_owned())
                        .font(k.ty.body_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(detail.to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_muted),
                ),
        );
    if let Some(action) = action {
        line = line.child(action);
    }
    line
}
