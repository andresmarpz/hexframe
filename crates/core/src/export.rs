use std::path::PathBuf;

use crate::library::{VideoInfo, sanitize};

pub const DISCORD_LIMIT_BYTES: u64 = 10_000_000;
pub const AUDIO_BITRATE: u32 = 128_000;
pub const MIN_SECS: f64 = 1.0;
pub const MOMENT_SECS: f64 = 15.0;
pub const PAD_SECS: f64 = 30.0;
pub const EXPORTS_DIR: &str = "exports";

const FIRST_TRY_PERCENT: u64 = 90;
const RETRY_PERCENT: u64 = 80;
const MIN_VIDEO_BITRATE: u32 = 120_000;
const HIGH_FPS_FROM: u32 = 3_500_000;
const HD_FROM: u32 = 700_000;
const SD_FROM: u32 = 350_000;
const RETRY_SCALE_MIN: f64 = 0.3;
const RETRY_SCALE_MAX: f64 = 0.95;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportPreset {
    #[default]
    Original,
    Discord,
}

impl ExportPreset {
    pub const ALL: [ExportPreset; 2] = [ExportPreset::Original, ExportPreset::Discord];

    pub fn label(self) -> &'static str {
        match self {
            ExportPreset::Original => "Original quality",
            ExportPreset::Discord => "For Discord (under 10 MB)",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            ExportPreset::Original => {
                "Copies the video without re-encoding. It cuts at keyframes, so it can start up to a second early."
            }
            ExportPreset::Discord => {
                "Re-encodes to 720p or less so the file fits under 10 MB. This takes a little while."
            }
        }
    }

    fn file_tag(self) -> &'static str {
        match self {
            ExportPreset::Original => "Original",
            ExportPreset::Discord => "Discord",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExportRange {
    pub start: f64,
    pub end: f64,
}

impl ExportRange {
    pub fn new(start: f64, end: f64) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn duration(self) -> f64 {
        self.end - self.start
    }

    pub fn moment(at: f64, total: f64) -> Self {
        let total = total.max(0.0);
        let mut start = at - MOMENT_SECS;
        let mut end = at + MOMENT_SECS;
        if start < 0.0 {
            end -= start;
            start = 0.0;
        }
        if end > total {
            start = (start - (end - total)).max(0.0);
            end = total;
        }
        Self { start, end }
    }

    pub fn bounds(self, total: f64) -> (f64, f64) {
        let lo = (self.start - PAD_SECS).max(0.0);
        let hi = (self.end + PAD_SECS).min(total.max(self.end));
        (lo, hi)
    }

    pub fn with_start(self, start: f64, bounds: (f64, f64)) -> Self {
        let latest = (self.end - MIN_SECS).max(bounds.0);
        Self {
            start: start.clamp(bounds.0, latest),
            end: self.end,
        }
    }

    pub fn with_end(self, end: f64, bounds: (f64, f64)) -> Self {
        let earliest = (self.start + MIN_SECS).min(bounds.1);
        Self {
            start: self.start,
            end: end.clamp(earliest, bounds.1),
        }
    }
}

pub fn file_stem(champion: &str, range: ExportRange, preset: ExportPreset) -> String {
    format!(
        "{}_{}_{}_{}",
        sanitize(champion, "Game"),
        clock(range.start),
        clock(range.end),
        preset.file_tag()
    )
}

fn clock(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    format!("{:02}-{:02}", total / 60, total % 60)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportJob {
    pub source: PathBuf,
    pub source_offset_secs: f64,
    pub range: ExportRange,
    pub preset: ExportPreset,
    pub video: VideoInfo,
    pub dest_dir: PathBuf,
    pub stem: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportDone {
    pub path: PathBuf,
    pub bytes: u64,
    pub range: ExportRange,
    pub preset: ExportPreset,
    pub shrunk: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportEvent {
    Progress(f32),
    Shrinking,
    Done(ExportDone),
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodePlan {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub video_bps: u32,
    pub audio_bps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("That is too long to fit under 10 MB. Pick {max_secs} seconds or less.")]
    TooLong { max_secs: u32 },
}

fn budget_bits(percent: u64) -> f64 {
    (DISCORD_LIMIT_BYTES * percent / 100 * 8) as f64
}

pub fn max_discord_secs() -> u32 {
    let floor_bps = f64::from(MIN_VIDEO_BITRATE + AUDIO_BITRATE);
    (budget_bits(RETRY_PERCENT) / floor_bps) as u32
}

pub fn plan_discord(duration_secs: f64, source: VideoInfo) -> Result<EncodePlan, PlanError> {
    let secs = duration_secs.max(MIN_SECS);
    if secs > f64::from(max_discord_secs()) {
        return Err(PlanError::TooLong {
            max_secs: max_discord_secs(),
        });
    }
    let total_bps = budget_bits(FIRST_TRY_PERCENT) / secs;
    plan_for(total_bps - f64::from(AUDIO_BITRATE), source)
}

pub fn retry_plan(
    previous: EncodePlan,
    actual_bytes: u64,
    duration_secs: f64,
    source: VideoInfo,
) -> Result<EncodePlan, PlanError> {
    let secs = duration_secs.max(MIN_SECS);
    let audio_bits = f64::from(previous.audio_bps) * secs;
    let actual_video_bits = (actual_bytes as f64 * 8.0 - audio_bits).max(1.0);
    let wanted_video_bits = budget_bits(RETRY_PERCENT) - audio_bits;
    let scale = (wanted_video_bits / actual_video_bits).clamp(RETRY_SCALE_MIN, RETRY_SCALE_MAX);
    plan_for(f64::from(previous.video_bps) * scale, source)
}

fn plan_for(video_bps: f64, source: VideoInfo) -> Result<EncodePlan, PlanError> {
    if video_bps < f64::from(MIN_VIDEO_BITRATE) {
        return Err(PlanError::TooLong {
            max_secs: max_discord_secs(),
        });
    }
    let video_bps = video_bps as u32;
    let wanted_height = if video_bps >= HD_FROM {
        720
    } else if video_bps >= SD_FROM {
        480
    } else {
        360
    };
    let height = even(wanted_height.min(source.height.max(2)));
    let width = if source.height == 0 {
        even(height * 16 / 9)
    } else {
        even(
            ((u64::from(height) * u64::from(source.width)) as f64 / f64::from(source.height))
                .round() as u32,
        )
    };
    let fps = if source.fps >= 50 && video_bps >= HIGH_FPS_FROM {
        60
    } else if source.fps == 0 {
        30
    } else {
        source.fps.min(30)
    };
    Ok(EncodePlan {
        width,
        height,
        fps,
        video_bps,
        audio_bps: AUDIO_BITRATE,
    })
}

fn even(value: u32) -> u32 {
    (value & !1).max(2)
}

#[cfg(test)]
mod tests;
