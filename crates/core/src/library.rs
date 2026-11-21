use std::fs;
use std::io;
use std::path::Path;

use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::game::{GameEvent, GameInfo, GameResult};
use crate::health::RecordingHealth;
use crate::review::ReviewData;

pub mod thumb;

pub const META_FILE: &str = "game.json";
pub const META_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("library io: {0}")]
    Io(#[from] io::Error),
    #[error("library json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported sidecar version {0}")]
    Version(u32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRecord {
    #[serde(flatten)]
    pub info: GameInfo,
    pub result: GameResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEntry {
    pub id: u32,
    pub game_secs: f64,
    pub rec_secs: f64,
    pub kind: GameEvent,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipEntry {
    pub file: String,
    pub start_secs: f64,
    pub end_secs: f64,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordingMeta {
    pub version: u32,
    pub id: String,
    pub started_at: Zoned,
    pub duration_secs: f64,
    pub video: String,
    pub game: GameRecord,
    pub events: Vec<EventEntry>,
    pub clips: Vec<ClipEntry>,
    pub encoder: String,
    pub video_format: VideoInfo,
    #[serde(default, skip_serializing_if = "ReviewData::is_empty")]
    pub review: ReviewData,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<RecordingHealth>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub recovered: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub(crate) fn sanitize(text: &str, fallback: &str) -> String {
    let clean: String = text
        .chars()
        .filter_map(|c| match c {
            '+' => Some('-'),
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => Some(c),
            _ => None,
        })
        .collect();
    if clean.is_empty() {
        fallback.to_owned()
    } else {
        clean
    }
}

pub fn folder_name(started: Zoned, champion: &str, mode: &str) -> String {
    format!(
        "{}_{}_{}",
        started.strftime("%Y-%m-%d_%H-%M-%S"),
        sanitize(champion, "Unknown"),
        sanitize(mode, "Unknown"),
    )
}

pub fn clip_file_name(rec_secs: f64, label: &str) -> String {
    let total = rec_secs.max(0.0) as u64;
    format!(
        "{:02}-{:02}_{}.mp4",
        total / 60,
        total % 60,
        sanitize(label, "Clip")
    )
}

pub fn write_meta(dir: &Path, meta: &RecordingMeta) -> Result<(), LibraryError> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{META_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_vec_pretty(meta)?)?;
    fs::rename(&tmp, dir.join(META_FILE))?;
    Ok(())
}

pub fn read_meta(dir: &Path) -> Result<RecordingMeta, LibraryError> {
    let meta: RecordingMeta = serde_json::from_slice(&fs::read(dir.join(META_FILE))?)?;
    if meta.version != META_VERSION {
        return Err(LibraryError::Version(meta.version));
    }
    Ok(meta)
}

pub fn write_review(dir: &Path, review: &ReviewData) -> Result<(), LibraryError> {
    let mut meta = read_meta(dir)?;
    meta.review = review.clone();
    write_meta(dir, &meta)
}

pub fn scan(output_dir: &Path) -> Vec<RecordingMeta> {
    let Ok(entries) = fs::read_dir(output_dir) else {
        return Vec::new();
    };
    let mut metas = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.join(META_FILE).is_file() {
            continue;
        }
        match read_meta(&dir) {
            Ok(meta) => metas.push(meta),
            Err(err) => tracing::warn!(dir = %dir.display(), %err, "skipping unreadable recording"),
        }
    }
    metas.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    metas
}

#[cfg(test)]
mod tests;
