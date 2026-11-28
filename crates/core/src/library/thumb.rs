use super::{ClipEntry, RecordingMeta};

pub const GAME_THUMB_FILE: &str = "thumb.jpg";
pub const GAME_HERO_FILE: &str = "hero.jpg";

const CLIP_MOMENT: f64 = 0.7;
const GAME_FALLBACK_MOMENT: f64 = 0.4;
const EDGE_SECS: f64 = 1.0;

pub fn clip_thumb_file(clip: &ClipEntry) -> String {
    let stem = clip.file.strip_suffix(".mp4").unwrap_or(&clip.file);
    format!("{stem}.jpg")
}

pub fn clip_thumb_secs(clip: &ClipEntry) -> f64 {
    let length = (clip.end_secs - clip.start_secs).max(0.0);
    (length * CLIP_MOMENT).min((length - EDGE_SECS).max(0.0))
}

pub fn game_thumb_secs(meta: &RecordingMeta) -> f64 {
    let last = (meta.duration_secs - EDGE_SECS).max(0.0);
    let moment = match meta.clips.first() {
        Some(clip) => clip.start_secs + clip_thumb_secs(clip),
        None => meta.duration_secs * GAME_FALLBACK_MOMENT,
    };
    moment.clamp(0.0, last)
}
