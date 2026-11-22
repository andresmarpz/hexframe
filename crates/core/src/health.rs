use serde::{Deserialize, Serialize};

use crate::time::Hns;

mod caps;
pub mod gpu;
mod stall;
mod stats;
mod tracker;
pub mod verdict;

pub use caps::{BufferUnit, CapMismatch, buffer_size, cap_mismatch, cap_warning};
pub use stall::{STALL_MIN_SECS, StallWatch};
pub use stats::{
    CaptureSetup, CaptureStats, LatencyHistogram, SpinShare, WakeLateness, driver_version_text,
};
pub use tracker::{DiagSample, HealthTracker, Queues};
pub use verdict::{Area, Finding, Level, audio_level};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RecordingHealth {
    pub target_fps: u32,
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64,
    pub frames_expected: u64,
    pub frames_captured: u64,
    pub frames_encoded: u64,
    pub frames_dropped: u64,
    pub frames_repeated: u64,
    pub dropped_pct: f64,
    pub repeated_pct: f64,
    pub source_fps: f64,
    pub longest_video_gap_ms: f64,
    pub encoder: String,
    pub encode_ms: Option<EncodeTiming>,
    pub audio: Option<AudioHealth>,
    pub file_bytes: u64,
    pub avg_bitrate_kbps: u32,
    pub capture_errors: u32,
    pub capture_soft_errors: u64,
    pub frames_torn_dropped: u64,
    pub copy_wait_slow: u64,
    pub copy_wait_timeouts: u64,
    pub video_stopped_secs: Option<f64>,
    pub frame_stalls: u32,
    pub longest_frame_stall_secs: f64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EncodeTiming {
    pub avg: f64,
    pub p95: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AudioHealth {
    pub packets: u64,
    pub gaps: u32,
    pub longest_gap_ms: f64,
    pub discontinuities: u64,
    pub idle_silence_ms: u64,
    pub start_offset_ms: f64,
    pub drift_ms: f64,
    pub timestamp_gaps_over_5s: u32,
    pub largest_timestamp_gap_ms: u64,
    pub trims: u32,
    pub trimmed_ms: u64,
    pub longest_trim_ms: u32,
}

impl RecordingHealth {
    pub fn effective_fps(&self) -> f64 {
        if self.duration_secs > 0.0 {
            self.frames_encoded as f64 / self.duration_secs
        } else {
            0.0
        }
    }

    pub fn findings(&self) -> Vec<Finding> {
        verdict::findings(self)
    }

    pub fn worst(&self) -> Level {
        self.findings()
            .iter()
            .map(|f| f.level)
            .max()
            .unwrap_or(Level::Ok)
    }

    pub fn summary_line(&self) -> String {
        let mut parts = vec![
            format!("{:.1} fps", self.effective_fps()),
            format!("{:.1}% dropped", self.dropped_pct),
        ];
        if let Some(audio) = &self.audio {
            let level = verdict::audio_level(audio);
            parts.push(match level {
                Level::Ok => "audio OK".to_owned(),
                Level::Warn => "audio warning".to_owned(),
                Level::Bad => "audio bad".to_owned(),
            });
        }
        let mbps = format!("{:.1}", f64::from(self.avg_bitrate_kbps) / 1000.0);
        parts.push(format!("{} Mbps", mbps.trim_end_matches(".0")));
        parts.join(" · ")
    }
}

pub const HEALTH_SINCE: &str = "0.1.12-preview.14";

pub fn no_health_text() -> String {
    format!("No health data (recorded before {HEALTH_SINCE})")
}

pub fn ticks_to_ms(ticks: i64, timescale: u32) -> f64 {
    ticks as f64 * 1000.0 / f64::from(timescale.max(1))
}

pub fn hns_ms(hns: Hns) -> f64 {
    hns as f64 / 10_000.0
}

#[cfg(test)]
mod tests;
