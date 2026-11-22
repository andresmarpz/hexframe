use std::time::Duration;

use greyscreen_engine::RecordingEntry;
use greyscreen_mp4::demux::Movie;

use super::SAVE_GRACE;
use super::check::CycleResult;

pub(super) fn missing_recording(index: u32, waited: Duration) -> CycleResult {
    CycleResult {
        index,
        folder: String::new(),
        video_secs: 0.0,
        dropped_pct: 0.0,
        capture_errors: 0,
        diag_seconds: Vec::new(),
        health: None,
        problem: Some(format!(
            "no saved recording {} s after the start",
            (waited + SAVE_GRACE).as_secs()
        )),
    }
}

pub(super) fn judge_recording(index: u32, entry: &RecordingEntry, min_secs: f64) -> CycleResult {
    let health = entry.meta.health.as_ref();
    let (video_secs, problem) = match playable_secs(entry, min_secs) {
        Ok(secs) => (secs, None),
        Err(problem) => (0.0, Some(problem)),
    };
    CycleResult {
        index,
        folder: entry.folder.display().to_string(),
        video_secs,
        dropped_pct: health.map_or(0.0, |h| h.dropped_pct),
        capture_errors: health.map_or(0, |h| h.capture_errors),
        problem,
        health: entry.meta.health.clone(),
        diag_seconds: diag_seconds(&entry.folder),
    }
}

fn diag_seconds(folder: &std::path::Path) -> Vec<serde_json::Value> {
    let Ok(text) = std::fs::read_to_string(folder.join("diagnostics.jsonl")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|v| v["type"] == "second")
        .collect()
}

fn playable_secs(entry: &RecordingEntry, min_secs: f64) -> Result<f64, String> {
    if entry.meta.video.is_empty() {
        return Err("the recording has no video file".to_owned());
    }
    let path = entry.folder.join(&entry.meta.video);
    let movie = Movie::open(&path).map_err(|e| format!("{} does not open: {e}", path.display()))?;
    let secs = movie.video_secs();
    if movie.video_samples.is_empty() || secs < min_secs {
        return Err(format!(
            "video is {secs:.1} s, expected at least {min_secs:.1} s"
        ));
    }
    Ok(secs)
}
