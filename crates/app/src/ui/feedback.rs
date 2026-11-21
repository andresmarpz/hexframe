use greyscreen_core::feedback::{
    DraftProblem, FeedbackDraft, FeedbackKind, FeedbackResult, MAX_MESSAGE_CHARS, new_client_id,
};
use greyscreen_engine::EngineCmd;
use rune::{TextEdit, Tx};

use super::Ui;

pub(crate) const MESSAGE_KEY: &str = "feedback-message";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum FeedbackPhase {
    #[default]
    Editing,
    Sending,
    Sent,
    Failed {
        error: String,
        report: String,
        copied: bool,
    },
}

#[derive(Debug)]
pub(crate) struct FeedbackUi {
    pub open: bool,
    pub kind: FeedbackKind,
    pub message: TextEdit,
    pub email: TextEdit,
    pub include_logs: bool,
    pub phase: FeedbackPhase,
    pub problem: Option<DraftProblem>,
}

impl Default for FeedbackUi {
    fn default() -> Self {
        Self {
            open: false,
            kind: FeedbackKind::default(),
            message: TextEdit::default(),
            email: TextEdit::default(),
            include_logs: true,
            phase: FeedbackPhase::Editing,
            problem: None,
        }
    }
}

impl FeedbackUi {
    pub(crate) fn sending(&self) -> bool {
        self.phase == FeedbackPhase::Sending
    }

    pub(crate) fn message_chars(&self) -> usize {
        self.message.text.chars().count()
    }

    fn reset_form(&mut self) {
        self.message.clear();
        self.email.clear();
        self.problem = None;
        self.phase = FeedbackPhase::Editing;
    }
}

impl Ui {
    pub(crate) fn open_feedback(&mut self, tx: &mut Tx) {
        self.close_settings();
        let feedback = &mut self.feedback;
        match feedback.phase {
            FeedbackPhase::Sent => feedback.reset_form(),
            FeedbackPhase::Failed { .. } => feedback.phase = FeedbackPhase::Editing,
            FeedbackPhase::Editing | FeedbackPhase::Sending => {}
        }
        feedback.open = true;
        tx.focus(MESSAGE_KEY, true);
    }

    pub(crate) fn close_feedback(&mut self) {
        let feedback = &mut self.feedback;
        if feedback.sending() {
            return;
        }
        feedback.open = false;
        match feedback.phase {
            FeedbackPhase::Sent => feedback.reset_form(),
            _ => feedback.phase = FeedbackPhase::Editing,
        }
    }

    pub(crate) fn set_feedback_kind(&mut self, index: usize) {
        if let Some(kind) = FeedbackKind::ALL.get(index) {
            self.feedback.kind = *kind;
        }
    }

    pub(crate) fn toggle_feedback_logs(&mut self) {
        self.feedback.include_logs = !self.feedback.include_logs;
    }

    pub(crate) fn feedback_message_edited(&mut self) {
        let feedback = &mut self.feedback;
        if feedback.message_chars() > MAX_MESSAGE_CHARS {
            let kept: String = feedback
                .message
                .text
                .chars()
                .take(MAX_MESSAGE_CHARS)
                .collect();
            feedback.message.replace_text(kept);
        }
        feedback.problem = None;
    }

    pub(crate) fn feedback_email_edited(&mut self) {
        self.feedback.problem = None;
    }

    pub(crate) fn submit_feedback(&mut self) {
        if matches!(
            self.feedback.phase,
            FeedbackPhase::Sending | FeedbackPhase::Sent
        ) {
            return;
        }
        if self.settings.client_id.is_none() {
            self.change(|settings| settings.client_id = Some(new_client_id()));
        }
        let draft = FeedbackDraft {
            kind: self.feedback.kind,
            message: self.feedback.message.text.clone(),
            email: self.feedback.email.text.clone(),
            include_logs: self.feedback.include_logs,
            client_id: self.settings.client_id.clone().unwrap_or_default(),
        };
        if let Some(problem) = draft.problem() {
            self.feedback.problem = Some(problem);
            return;
        }
        self.feedback.problem = None;
        self.feedback.phase = FeedbackPhase::Sending;
        self.send(EngineCmd::SendFeedback(draft));
    }

    pub(crate) fn apply_feedback(&mut self, result: FeedbackResult) {
        if !self.feedback.sending() {
            return;
        }
        self.feedback.phase = match result {
            FeedbackResult::Sent => FeedbackPhase::Sent,
            FeedbackResult::Failed { error, report } => FeedbackPhase::Failed {
                error,
                report,
                copied: false,
            },
        };
    }

    pub(crate) fn copy_feedback_report(&mut self, tx: &mut Tx) {
        if let FeedbackPhase::Failed { report, copied, .. } = &mut self.feedback.phase {
            tx.copy(report.clone());
            *copied = true;
        }
    }
}
