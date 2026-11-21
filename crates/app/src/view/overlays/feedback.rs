use greyscreen_core::feedback::{DraftProblem, FeedbackKind, LOG_TAIL_LINES, MAX_MESSAGE_CHARS};
use rune::{El, ScrollSpec, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, Checkbox, ControlSize, Dialog, Field, Kit, SegmentedControl, Spinner,
    State, TextArea, TextInput,
};

use crate::ui::{FEEDBACK_MESSAGE_KEY, FeedbackPhase, FeedbackUi, Ui};

const DIALOG_WIDTH: f32 = 480.0;
const DIALOG_CHROME: f32 = 190.0;
const MIN_FIELDS_HEIGHT: f32 = 120.0;
const MESSAGE_ROWS: usize = 5;
const SUBTITLE: &str = "Tell us what went wrong or what you'd like to see.";
pub(crate) const THANKS: &str = "Thanks, we got it.";

pub(crate) fn dialog(s: &Ui, k: &Kit, layer: &mut Vec<El>, window_height: Option<f32>) {
    let feedback = &s.feedback;
    if !feedback.open {
        return;
    }
    let body = match feedback.phase {
        FeedbackPhase::Sent => thanks(k),
        _ => form(feedback, k, window_height),
    };
    Dialog::new("feedback", "Send feedback")
        .description(SUBTITLE)
        .width(DIALOG_WIDTH)
        .fit()
        .open(true)
        .on_close(|ui: &mut Ui| ui.close_feedback())
        .child(body)
        .build(k, layer);
}

fn thanks(k: &Kit) -> El {
    col()
        .key("feedback-body")
        .w_full()
        .px(16.0)
        .pb(16.0)
        .gap(16.0)
        .child(
            text(THANKS)
                .key("feedback-thanks")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            row().w_full().justify_end().child(
                Button::new("Close")
                    .variant(ButtonVariant::Contrast)
                    .control_size(ControlSize::Large)
                    .on_press(|ui: &mut Ui| ui.close_feedback())
                    .build(k)
                    .key("feedback-close"),
            ),
        )
}

fn form(f: &FeedbackUi, k: &Kit, window_height: Option<f32>) -> El {
    let fields_height =
        window_height.map_or(f32::MAX, |h| (h - DIALOG_CHROME).max(MIN_FIELDS_HEIGHT));
    col()
        .key("feedback-body")
        .w_full()
        .child(
            col()
                .key("feedback-fields")
                .w_full()
                .max_h(fields_height)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    col()
                        .w_full()
                        .px(16.0)
                        .pb(16.0)
                        .gap(16.0)
                        .child(kind(f, k))
                        .child(message(f, k))
                        .child(email(f, k))
                        .child(logs(f, k)),
                ),
        )
        .child(footer(f, k))
}

fn kind(f: &FeedbackUi, k: &Kit) -> El {
    let labels: Vec<&str> = FeedbackKind::ALL.iter().map(|kind| kind.label()).collect();
    let selected = FeedbackKind::ALL.iter().position(|kind| *kind == f.kind);
    let mut control = SegmentedControl::new(&labels, selected)
        .label("Kind")
        .on_pick(|ui: &mut Ui, index, _: &mut Tx| ui.set_feedback_kind(index));
    if f.sending() {
        for index in 0..labels.len() {
            control = control.segment_state(index, State::Disabled);
        }
    }
    Field::new()
        .label("Kind")
        .control(control.build(k).key("feedback-kind"))
        .build(k)
}

fn message(f: &FeedbackUi, k: &Kit) -> El {
    let invalid = matches!(
        f.problem,
        Some(DraftProblem::EmptyMessage | DraftProblem::MessageTooLong)
    );
    let area = TextArea::new(&f.message)
        .label("Message")
        .placeholder("What happened, or what would help?")
        .rows(MESSAGE_ROWS)
        .invalid(invalid)
        .disabled(f.sending())
        .on_edit(|ui: &mut Ui| &mut ui.feedback.message)
        .on_change(|ui: &mut Ui, _: &mut Tx| ui.feedback_message_edited())
        .on_submit(|ui: &mut Ui, _: &mut Tx| ui.submit_feedback())
        .build(k)
        .w_full()
        .key(FEEDBACK_MESSAGE_KEY);
    Field::new()
        .label("Message")
        .suffix(format!("{} / {MAX_MESSAGE_CHARS}", f.message_chars()))
        .control(area)
        .build(k)
}

fn email(f: &FeedbackUi, k: &Kit) -> El {
    let input = TextInput::new(&f.email)
        .label("Email")
        .placeholder("you@example.com")
        .width(None)
        .invalid(f.problem == Some(DraftProblem::BadEmail))
        .disabled(f.sending())
        .on_edit(|ui: &mut Ui| &mut ui.feedback.email)
        .on_change(|ui: &mut Ui, _: &mut Tx| ui.feedback_email_edited())
        .on_submit(|ui: &mut Ui, _: &mut Tx| ui.submit_feedback())
        .build(k)
        .w_full()
        .key("feedback-email");
    Field::new()
        .label("Email (optional)")
        .suffix("If you want a reply.")
        .control(input)
        .build(k)
}

fn logs(f: &FeedbackUi, k: &Kit) -> El {
    Checkbox::new(f.include_logs)
        .label("Include recent logs")
        .description(format!(
            "Sends the last {LOG_TAIL_LINES} log lines. Your user folder name is removed."
        ))
        .disabled(f.sending())
        .on_toggle(|ui: &mut Ui| ui.toggle_feedback_logs())
        .build(k)
        .key("feedback-logs")
}

fn status(f: &FeedbackUi, k: &Kit) -> Option<El> {
    let line = |key: &'static str, message: String| {
        text(message)
            .key(key)
            .wrap()
            .w_full()
            .font(k.ty.body)
            .color(k.c.danger.fg)
    };
    match (&f.phase, f.problem) {
        (FeedbackPhase::Failed { error, .. }, _) => Some(line("feedback-error", error.clone())),
        (_, Some(problem)) => Some(line("feedback-problem", problem.text().to_owned())),
        _ => None,
    }
}

fn footer(f: &FeedbackUi, k: &Kit) -> El {
    let sending = f.sending();
    let mut buttons = row().w_full().gap(8.0).center_y().justify_end();
    if sending {
        buttons = buttons
            .child(Spinner::new().size(16.0).build(k).key("feedback-spinner"))
            .child(
                text("Sending…")
                    .key("feedback-sending")
                    .font(k.ty.body)
                    .color(k.c.text_muted),
            )
            .child(col().grow(1.0).a11y_hidden());
    }
    if let FeedbackPhase::Failed { report, copied, .. } = &f.phase
        && !report.is_empty()
    {
        let label = if *copied { "Copied" } else { "Copy report" };
        buttons = buttons.child(
            Button::new(label)
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.copy_feedback_report(tx))
                .build(k)
                .key("feedback-copy"),
        );
    }
    buttons = buttons
        .child(
            Button::new("Cancel")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .disabled(sending)
                .on_press(|ui: &mut Ui| ui.close_feedback())
                .build(k)
                .key("feedback-cancel"),
        )
        .child(
            Button::new("Send")
                .variant(ButtonVariant::Contrast)
                .control_size(ControlSize::Large)
                .disabled(sending)
                .on_press(|ui: &mut Ui| ui.submit_feedback())
                .build(k)
                .key("feedback-send"),
        );
    col()
        .key("feedback-footer")
        .w_full()
        .px(16.0)
        .pb(16.0)
        .gap(12.0)
        .children(status(f, k))
        .child(buttons)
}
