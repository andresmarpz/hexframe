use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

mod scrub;

pub use scrub::home_variants;
use scrub::scrub_logs;

pub const MAX_MESSAGE_CHARS: usize = 5000;
pub const LOG_TAIL_LINES: usize = 300;
const LOG_FILE_PREFIX: &str = "greyscreen.log.";
const MAX_LOG_READ_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackKind {
    #[default]
    Bug,
    Idea,
    Other,
}

impl FeedbackKind {
    pub const ALL: [FeedbackKind; 3] = [FeedbackKind::Bug, FeedbackKind::Idea, FeedbackKind::Other];

    pub fn label(self) -> &'static str {
        match self {
            FeedbackKind::Bug => "Bug",
            FeedbackKind::Idea => "Idea",
            FeedbackKind::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftProblem {
    EmptyMessage,
    MessageTooLong,
    BadEmail,
}

impl DraftProblem {
    pub fn text(self) -> &'static str {
        match self {
            DraftProblem::EmptyMessage => "Write a message first.",
            DraftProblem::MessageTooLong => "That message is too long.",
            DraftProblem::BadEmail => "That email address doesn't look right.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackDraft {
    pub kind: FeedbackKind,
    pub message: String,
    pub email: String,
    pub include_logs: bool,
    pub client_id: String,
}

impl FeedbackDraft {
    pub fn problem(&self) -> Option<DraftProblem> {
        let message = self.message.trim();
        if message.is_empty() {
            return Some(DraftProblem::EmptyMessage);
        }
        if message.chars().count() > MAX_MESSAGE_CHARS {
            return Some(DraftProblem::MessageTooLong);
        }
        let email = self.email.trim();
        (!email.is_empty() && !looks_like_email(email)).then_some(DraftProblem::BadEmail)
    }
}

fn looks_like_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.chars().any(char::is_whitespace)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeedbackReport {
    pub kind: FeedbackKind,
    pub message: String,
    pub email: Option<String>,
    pub app_version: String,
    pub os: String,
    pub log_tail: Option<String>,
    pub client_id: String,
}

impl FeedbackReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackResult {
    Sent,
    Failed { error: String, report: String },
}

#[derive(Debug, Clone, Copy)]
pub struct ReportContext<'a> {
    pub app_version: &'a str,
    pub os: &'a str,
    pub home_dirs: &'a [String],
}

pub fn build_report(
    draft: &FeedbackDraft,
    context: ReportContext<'_>,
    raw_log: impl FnOnce() -> Option<String>,
) -> FeedbackReport {
    let email = draft.email.trim();
    let log_tail = if draft.include_logs {
        raw_log().map(|raw| scrub_logs(&raw, context.home_dirs))
    } else {
        None
    };
    FeedbackReport {
        kind: draft.kind,
        message: draft.message.trim().to_owned(),
        email: (!email.is_empty()).then(|| email.to_owned()),
        app_version: context.app_version.to_owned(),
        os: context.os.to_owned(),
        log_tail,
        client_id: draft.client_id.clone(),
    }
}

pub fn read_newest_log(dir: &Path) -> Option<String> {
    let newest = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(LOG_FILE_PREFIX)
        })
        .max_by_key(|entry| entry.file_name())?;
    read_end(&newest.path()).ok()
}

fn read_end(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(MAX_LOG_READ_BYTES)))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn new_client_id() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::fill(&mut bytes).is_err() {
        bytes = fallback_bytes();
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn fallback_bytes() -> [u8; 16] {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let mut state = (nanos as u64) ^ u64::from(std::process::id()).rotate_left(32) | 1;
    let mut bytes = [0u8; 16];
    for byte in &mut bytes {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = (state >> 24) as u8;
    }
    bytes
}

#[cfg(test)]
mod tests;
