use greyscreen_engine::{EngineStatus, RecordingEntry};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::ui::{TEST_RECORDING_SECS, TestRun, Ui, format_clock};
use crate::view::settings::quality_control;
use crate::view::widgets::busy_label_content;
use crate::view::{card, thumb};

const THUMB_WIDTH: f32 = 96.0;
const TEST_BUTTON_LABEL: &str = "Test recording (10 s)";

pub(super) fn card_for(s: &Ui, k: &Kit) -> El {
    card(k)
        .key("first-run")
        .padding(20.0)
        .gap(16.0)
        .child(
            col()
                .gap(2.0)
                .child(
                    text("Set up Greyscreen")
                        .font(k.ty.body_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text("Three quick checks before your first game.")
                        .font(k.ty.body)
                        .color(k.c.text_muted),
                ),
        )
        .child(folder_step(s, k))
        .child(quality_step(s, k))
        .child(test_step(s, k))
        .child(
            row().w_full().justify_end().child(
                Button::new("Done")
                    .variant(ButtonVariant::Neutral)
                    .control_size(ControlSize::Large)
                    .on_press(|ui: &mut Ui| ui.finish_first_run())
                    .build(k)
                    .key("first-run-done"),
            ),
        )
}

fn folder_step(s: &Ui, k: &Kit) -> El {
    let change = Button::new("Change…")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.choose_folder(tx))
        .build(k)
        .key("first-run-folder");
    step(
        k,
        1,
        "Where recordings are saved",
        text(s.settings.output_dir.display().to_string())
            .key("first-run-path")
            .font(k.ty.small)
            .color(k.c.text_muted)
            .truncate(),
        Some(change),
    )
}

fn quality_step(s: &Ui, k: &Kit) -> El {
    step(
        k,
        2,
        "Quality",
        text("Higher quality uses more disk space.")
            .font(k.ty.small)
            .color(k.c.text_muted),
        Some(quality_control(s, k, "first-run-quality")),
    )
}

fn test_step(s: &Ui, k: &Kit) -> El {
    match &s.first_test {
        TestRun::Passed(entry) => passed(entry, k),
        TestRun::Running => {
            let kit = k.clone();
            let button = Button::new(TEST_BUTTON_LABEL)
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Large)
                .disabled(true)
                .label_with(move |font, ink| {
                    busy_label_content(
                        &kit,
                        TEST_BUTTON_LABEL,
                        true,
                        "first-run-test-spinner",
                        font,
                        ink,
                    )
                })
                .build(k)
                .key("first-run-test");
            step(
                k,
                3,
                "Test recording",
                text(format!("Recording for {TEST_RECORDING_SECS} seconds…"))
                    .key("first-run-testing")
                    .font(k.ty.small)
                    .color(k.c.text_muted),
                Some(button),
            )
        }
        run => {
            let testable = matches!(s.status, EngineStatus::Idle | EngineStatus::GameRunning);
            let button = Button::new(TEST_BUTTON_LABEL)
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Large)
                .disabled(!testable)
                .on_press(|ui: &mut Ui| ui.run_first_test())
                .build(k)
                .key("first-run-test");
            let detail = if *run == TestRun::Failed {
                text("That did not work. See the message above.")
                    .key("first-run-failed")
                    .font(k.ty.small)
                    .color(k.c.danger.fg)
            } else {
                text("Check that video and sound are saved.")
                    .font(k.ty.small)
                    .color(k.c.text_muted)
            };
            step(k, 3, "Test recording", detail, Some(button))
        }
    }
}

fn passed(entry: &RecordingEntry, k: &Kit) -> El {
    let length = format_clock(entry.meta.duration_secs as u32);
    let watch = Button::new("Watch")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.open_last_game())
        .build(k)
        .key("first-run-watch");
    row()
        .key("first-run-passed")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(thumb::game_thumb(k, entry, THUMB_WIDTH))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text("It works")
                        .font(k.ty.body_medium)
                        .color(k.c.success.fg),
                )
                .child(
                    text(format!("Saved {length} with video and sound."))
                        .font(k.ty.small)
                        .color(k.c.text_muted),
                ),
        )
        .child(watch)
}

fn step(k: &Kit, number: u32, title: &str, detail: El, action: Option<El>) -> El {
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
                .child(detail),
        );
    if let Some(action) = action {
        line = line.child(action);
    }
    line
}
