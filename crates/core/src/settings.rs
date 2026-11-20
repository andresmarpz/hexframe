use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::media::CaptureConfig;
use crate::paths;

const AUDIO_BITRATE: u32 = 192_000;
const REFERENCE_PIXELS: f64 = 1920.0 * 1080.0;
const REFERENCE_FPS: f64 = 60.0;
const PEAK_FACTOR: f64 = 1.5;
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
            Quality::Balanced => 12_000_000.0,
            Quality::High => 20_000_000.0,
            Quality::Max => 35_000_000.0,
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

    pub fn capture_config(&self, native_height: u32) -> CaptureConfig {
        let max_height = self.resolution.height().map(|h| h.min(native_height));
        let height = max_height.unwrap_or(native_height);
        let width = f64::from(height) * 16.0 / 9.0;
        let pixels = width * f64::from(height);
        let mean = self.quality.mean_bitrate_at_1080p60()
            * (pixels / REFERENCE_PIXELS)
            * (f64::from(self.fps) / REFERENCE_FPS);
        CaptureConfig {
            fps: self.fps,
            max_height,
            video_bitrate: mean.round() as u32,
            video_peak: (mean * PEAK_FACTOR).round() as u32,
            gop_frames: self.fps,
            audio: self.audio,
            audio_bitrate: AUDIO_BITRATE,
        }
    }
}

#[cfg(test)]
mod tests;
