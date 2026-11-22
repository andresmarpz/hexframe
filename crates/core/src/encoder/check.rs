use serde::{Deserialize, Serialize};

use super::{
    BUSY_HELP, EncoderKind, FAILED_HELP, Probe, Reason, SOFTWARE_NOTICE, VENDOR_NVIDIA, Verdict,
    label,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub kind: EncoderKind,
    pub adapter: String,
    pub vendor_id: u32,
    pub result: Result<String, String>,
    pub reason: Reason,
}

impl Attempt {
    pub fn label(&self) -> String {
        label(self.kind, self.vendor_id, &self.adapter)
    }

    fn failure_text(&self, error: &str) -> String {
        let label = self.label();
        if self.adapter.is_empty() || label.contains(&self.adapter) {
            format!("{label}: {error}")
        } else {
            format!("{label} on {}: {error}", self.adapter)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Hardware,
    Software,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderCheck {
    pub key: String,
    pub outcome: CheckOutcome,
    pub encoder: String,
    pub detail: String,
    #[serde(default)]
    pub first_failure: String,
    #[serde(default)]
    pub noticed: bool,
    #[serde(default)]
    pub verdicts: Vec<Verdict>,
}

impl EncoderCheck {
    pub fn from_attempts(key: String, attempts: &[Attempt]) -> Self {
        let lines = attempts
            .iter()
            .filter_map(|a| a.result.as_ref().err().map(|e| (a, e)))
            .collect::<Vec<_>>();
        let failures = lines
            .iter()
            .map(|(a, e)| format!("{}: {e}", a.label()))
            .collect::<Vec<_>>()
            .join("; ");
        let chosen = attempts.iter().find(|a| a.result.is_ok());
        let (outcome, encoder) = match chosen {
            Some(a) if a.kind == EncoderKind::Hardware => (CheckOutcome::Hardware, a.label()),
            Some(a) => (CheckOutcome::Software, a.label()),
            None => (CheckOutcome::Failed, String::new()),
        };
        Self {
            key,
            outcome,
            encoder,
            detail: failures,
            first_failure: lines
                .first()
                .map(|(a, e)| a.failure_text(e))
                .unwrap_or_default(),
            noticed: false,
            verdicts: Vec::new(),
        }
    }

    pub fn reusable_for(&self, key: &str) -> bool {
        self.key == key
            && match self.outcome {
                CheckOutcome::Failed => false,
                CheckOutcome::Hardware => true,
                CheckOutcome::Software => !self.has_unsure_verdict(),
            }
    }

    pub fn has_unsure_verdict(&self) -> bool {
        self.verdicts
            .iter()
            .any(|v| v.reason.is_some_and(Reason::is_unsure))
    }

    pub fn crashed_before(&self, probe: &Probe) -> bool {
        self.verdicts
            .iter()
            .any(|v| v.reason == Some(Reason::Crashed) && v.probe.same_target(probe))
    }

    pub fn settings_line(&self) -> String {
        match self.outcome {
            CheckOutcome::Hardware => self.encoder.clone(),
            CheckOutcome::Software => format!("{} — uses more CPU", self.encoder),
            CheckOutcome::Failed => "None works".to_owned(),
        }
    }

    pub fn wants_notice(&self) -> bool {
        self.outcome == CheckOutcome::Software && !self.noticed
    }

    pub fn busy_encoder(&self) -> bool {
        self.verdicts
            .iter()
            .any(|v| v.reason == Some(Reason::Busy) && v.probe.vendor_id == VENDOR_NVIDIA)
    }

    pub fn notice_text(&self) -> &'static str {
        if self.busy_encoder() {
            BUSY_HELP
        } else {
            SOFTWARE_NOTICE
        }
    }

    pub fn failed_help(&self) -> &'static str {
        if self.busy_encoder() {
            BUSY_HELP
        } else {
            FAILED_HELP
        }
    }

    pub fn failure_line(&self) -> Option<&str> {
        Some(self.first_failure.as_str()).filter(|line| !line.is_empty())
    }

    pub fn failed(&self) -> bool {
        self.outcome == CheckOutcome::Failed
    }
}
