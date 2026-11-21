use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::media::CaptureConfig;
use crate::paths;

const AUDIO_BITRATE: u32 = 192_000;
const REFERENCE_PIXELS: f64 = 1920.0 * 1080.0;
const REFERENCE_FPS: f64 = 60.0;
const PIXEL_EXPONENT: f64 = 0.75;
const FPS_EXPONENT: f64 = 0.8;
const PEAK_FACTOR: f64 = 2.0;
const QUALITY_MODE_SIZE_FACTOR: f64 = 0.6;
const ESTIMATE_NATIVE_HEIGHT: u32 = 1440;
const MAX_CLIP_SECS: u32 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Quality {
    Balanced,
    #[default]
    High,
    Max,
}

impl Quality {
    fn mean_bitrate_at_1080p60(self) -> f64 {
        match self {
            Quality::Balanced => 8_000_000.0,
            Quality::High => 12_000_000.0,
            Quality::Max => 20_000_000.0,
        }
    }

    fn target_quality(self) -> u32 {
        match self {
            Quality::Balanced => 60,
            Quality::High => 70,
            Quality::Max => 80,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Resolution {
    Native,
    P1440,
    #[default]
    P1080,
    P720,
}

impl Resolution {
    fn height(self) -> Option<u32> {
        match self {
            Resolution::Native => None,
            Resolution::P1440 => Some(1440),
            Resolution::P1080 => Some(1080),
            Resolution::P720 => Some(720),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipSettings {
    pub enabled: bool,
    pub pre_secs: u32,
    pub post_secs: u32,
    pub kills: bool,
    pub deaths: bool,
    pub assists: bool,
    pub multikills: bool,
    pub objectives: bool,
    pub structures: bool,
    pub aces: bool,
}

impl Default for ClipSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            pre_secs: 15,
            post_secs: 5,
            kills: true,
            deaths: true,
            assists: false,
            multikills: true,
            objectives: true,
            structures: false,
            aces: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output_dir: PathBuf,
    pub auto_record: bool,
    pub record_full_game: bool,
    pub quality: Quality,
    pub resolution: Resolution,
    pub fps: u32,
    pub audio: bool,
    pub capture_cursor: bool,
    pub clips: ClipSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: paths::default_output_dir(),
            auto_record: true,
            record_full_game: true,
            quality: Quality::default(),
            resolution: Resolution::default(),
            fps: 60,
            audio: true,
            capture_cursor: true,
            clips: ClipSettings::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    tracing::warn!(path = %path.display(), %err, "cannot read settings, using defaults");
                }
                return Settings::default();
            }
        };
        match serde_json::from_str(&text) {
            Ok(settings) => settings,
            Err(err) => {
                tracing::warn!(path = %path.display(), %err, "invalid settings file, using defaults");
                Settings::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)
    }

    pub fn validate(&mut self) {
        self.fps = if self.fps.abs_diff(30) < self.fps.abs_diff(60) {
            30
        } else {
            60
        };
        self.clips.pre_secs = self.clips.pre_secs.clamp(1, MAX_CLIP_SECS);
        self.clips.post_secs = self.clips.post_secs.clamp(1, MAX_CLIP_SECS);
    }

    fn mean_bitrate(&self, height: u32) -> (f64, f64) {
        let width = f64::from(height) * 16.0 / 9.0;
        let pixels = width * f64::from(height);
        let mean = self.quality.mean_bitrate_at_1080p60()
            * (pixels / REFERENCE_PIXELS).powf(PIXEL_EXPONENT)
            * (f64::from(self.fps) / REFERENCE_FPS).powf(FPS_EXPONENT);
        (mean, pixels)
    }

    pub fn capture_config(&self, native_height: u32) -> CaptureConfig {
        let max_height = self.resolution.height().map(|h| h.min(native_height));
        let height = max_height.unwrap_or(native_height);
        let (mean, pixels) = self.mean_bitrate(height);
        CaptureConfig {
            fps: self.fps,
            max_height,
            video_bitrate: mean.round() as u32,
            video_peak: (mean * PEAK_FACTOR).round() as u32,
            quality: Some(self.quality.target_quality()),
            gop_frames: self.fps,
            audio: self.audio,
            audio_bitrate: AUDIO_BITRATE,
            cursor: self.capture_cursor,
            bitrate_pixels: pixels.round() as u64,
        }
    }

    pub fn estimated_mb_per_minute(&self) -> f64 {
        let height = self.resolution.height().unwrap_or(ESTIMATE_NATIVE_HEIGHT);
        let (mean, _) = self.mean_bitrate(height);
        let video = mean * QUALITY_MODE_SIZE_FACTOR;
        let audio = if self.audio {
            f64::from(AUDIO_BITRATE)
        } else {
            0.0
        };
        (video + audio) * 60.0 / 8.0 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests;
