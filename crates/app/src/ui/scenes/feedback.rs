use rune::TextEdit;

use super::{Scene, home};
use crate::ui::{FeedbackPhase, Ui};

fn feedback(ui: &mut Ui) {
    home(ui);
    ui.feedback.open = true;
    ui.feedback.message =
        TextEdit::new("The Clip button does nothing when I press F9 during a game.");
}

pub(super) const SCENES: &[Scene] = &[
    ("feedback", feedback),
    ("feedback-sending", |ui| {
        feedback(ui);
        ui.feedback.email = TextEdit::new("ana@example.com");
        ui.feedback.phase = FeedbackPhase::Sending;
    }),
    ("feedback-sent", |ui| {
        feedback(ui);
        ui.feedback.phase = FeedbackPhase::Sent;
    }),
    ("feedback-error", |ui| {
        feedback(ui);
        ui.feedback.phase = FeedbackPhase::Failed {
            error: "Couldn't reach Greyscreen. Check your connection and try again.".into(),
            report: "{}".into(),
            copied: false,
        };
    }),
];
