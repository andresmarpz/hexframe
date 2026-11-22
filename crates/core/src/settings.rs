use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::categories::CategorySet;
use crate::keybinds::Keymap;
use crate::media::CaptureConfig;
use crate::paths;

mod overrides;
mod resolution;

pub use resolution::{Resolution, ResolutionOption, resolution_options};

const AUDIO_BITRATE: u32 = 192_000;
const REFERENCE_PIXELS: f64 = 1920.0 * 1080.0;
const REFERENCE_FPS: f64 = 60.0;
const PIXEL_EXPONENT: f64 = 0.75;
const FPS_EXPONENT: f64 = 0.8;
const PEAK_FACTOR: f64 = 2.0;
const QUALITY_MODE_SIZE_FACTOR: f64 = 0.6;
const MAX_CLIP_SECS: u32 = 120;
const BYTES_PER_GB: u64 = 1_000_000_000;
pub const MAX_SOUND_VOLUME: u32 = 100;
pub const NATIVE_HEIGHT_CEILING: u32 = 2160;
pub const DEFAULT_SOUND_VOLUME: u32 = 60;
pub const MIN_WINDOW_SIZE: (u32, u32) = (1000, 600);
pub const MAX_WINDOW_SIZE: (u32, u32) = (8192, 8192);
pub const STORAGE_LIMITS_GB: [u32; 4] = [50, 100, 250, 500];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneWidth {
    pub min: u32,
    pub max: u32,
    pub default: u32,
}

impl PaneWidth {
    pub fn clamp(self, width: u32) -> u32 {
        width.clamp(self.min, self.max)
    }
}

pub const NAV_WIDTH: PaneWidth = PaneWidth {
    min: 180,
    max: 320,
    default: 220,
};
pub const LIST_WIDTH: PaneWidth = PaneWidth {
    min: 240,
    max: 440,
    default: 300,
};

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
pub enum NavStyle {
    #[default]
    Sidebar,
    Dock,
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
    pub recording_diagnostics: bool,
    pub sound_effects: bool,
    pub sound_volume: u32,
    pub clips: ClipSettings,
    pub review_events: CategorySet,
    #[serde(deserialize_with = "overrides::lenient_overrides")]
    pub keybinds: BTreeMap<String, String>,
    #[serde(
        rename = "save_clip_hotkey",
        deserialize_with = "overrides::legacy_hotkey",
        skip_serializing
    )]
    pub legacy_save_clip_hotkey: Option<Option<String>>,
    pub storage_limit_gb: Option<u32>,
    pub check_updates: bool,
    pub nav_style: NavStyle,
    pub sidebar_collapsed: bool,
    pub review_inspector_hidden: bool,
    pub games_list_hidden: bool,
    pub nav_width: u32,
    pub list_width: u32,
    pub window_size: Option<(u32, u32)>,
    pub window_maximized: bool,
    pub client_id: Option<String>,
    #[serde(default = "yes")]
    pub first_run_done: bool,
}

fn yes() -> bool {
    true
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
            recording_diagnostics: false,
            sound_effects: true,
            sound_volume: DEFAULT_SOUND_VOLUME,
            clips: ClipSettings::default(),
            review_events: CategorySet::default(),
            keybinds: BTreeMap::new(),
            legacy_save_clip_hotkey: None,
            storage_limit_gb: None,
            check_updates: true,
            nav_style: NavStyle::default(),
            sidebar_collapsed: false,
            review_inspector_hidden: false,
            games_list_hidden: false,
            nav_width: NAV_WIDTH.default,
            list_width: LIST_WIDTH.default,
            window_size: None,
            window_maximized: false,
            client_id: None,
            first_run_done: false,
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
        match serde_json::from_str::<Settings>(&text) {
            Ok(mut settings) => {
                overrides::migrate_legacy(
                    &mut settings.legacy_save_clip_hotkey,
                    &mut settings.keybinds,
                );
                settings
            }
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
        self.resolution = self.resolution.clamped();
        self.clips.pre_secs = self.clips.pre_secs.clamp(1, MAX_CLIP_SECS);
        self.clips.post_secs = self.clips.post_secs.clamp(1, MAX_CLIP_SECS);
        self.storage_limit_gb = self.storage_limit_gb.map(nearest_storage_limit);
        self.sound_volume = self.sound_volume.min(MAX_SOUND_VOLUME);
        self.nav_width = NAV_WIDTH.clamp(self.nav_width);
        self.list_width = LIST_WIDTH.clamp(self.list_width);
        self.window_size = self.window_size.map(|(w, h)| {
            (
                w.clamp(MIN_WINDOW_SIZE.0, MAX_WINDOW_SIZE.0),
                h.clamp(MIN_WINDOW_SIZE.1, MAX_WINDOW_SIZE.1),
            )
        });
    }

    pub fn keymap(&self) -> Keymap {
        Keymap::from_overrides(&self.keybinds)
    }

    pub fn storage_limit_bytes(&self) -> Option<u64> {
        self.storage_limit_gb.map(|gb| u64::from(gb) * BYTES_PER_GB)
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
            diagnostics: self.recording_diagnostics,
        }
    }

    pub fn estimated_mb_per_minute(&self, native_height: u32) -> f64 {
        let height = self
            .resolution
            .height()
            .map_or(native_height, |h| h.min(native_height));
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

fn nearest_storage_limit(gb: u32) -> u32 {
    STORAGE_LIMITS_GB
        .into_iter()
        .min_by_key(|option| option.abs_diff(gb))
        .unwrap_or(STORAGE_LIMITS_GB[0])
}

#[cfg(test)]
mod tests;
