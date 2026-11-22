use serde::Serialize;

use super::{AudioHealth, RecordingHealth};

pub const DROPPED_WARN_PCT: f64 = 0.5;
pub const DROPPED_BAD_PCT: f64 = 2.0;
pub const REPEATED_WARN_PCT: f64 = 5.0;
pub const REPEATED_BAD_PCT: f64 = 20.0;
pub const GAP_WARN_MS: f64 = 100.0;
pub const GAP_BAD_MS: f64 = 500.0;
pub const AUDIO_GAP_WARN_MS: f64 = 50.0;
pub const AUDIO_GAP_BAD_MS: f64 = 250.0;
pub const DRIFT_WARN_MS: f64 = 50.0;
pub const DRIFT_BAD_MS: f64 = 250.0;
pub const OFFSET_WARN_MS: f64 = 50.0;
pub const OFFSET_BAD_MS: f64 = 250.0;
pub const STALL_BAD_SECS: f64 = 120.0;
pub const MIN_BITRATE_KBPS: u32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Warn,
    Bad,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Ok => "OK",
            Level::Warn => "WARNING",
            Level::Bad => "BAD",
        }
    }

    pub fn grade(value: f64, warn: f64, bad: f64) -> Level {
        if value >= bad {
            Level::Bad
        } else if value >= warn {
            Level::Warn
        } else {
            Level::Ok
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Area {
    Video,
    Pacing,
    Gaps,
    Encoder,
    Audio,
    Capture,
    File,
}

impl Area {
    pub fn label(self) -> &'static str {
        match self {
            Area::Video => "Video frames",
            Area::Pacing => "Frame pacing",
            Area::Gaps => "Video gaps",
            Area::Encoder => "Encoder",
            Area::Audio => "Audio",
            Area::Capture => "Capture",
            Area::File => "File",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    pub area: Area,
    pub level: Level,
    pub text: String,
}

pub fn audio_level(audio: &AudioHealth) -> Level {
    let gap = Level::grade(audio.longest_gap_ms, AUDIO_GAP_WARN_MS, AUDIO_GAP_BAD_MS);
    let drift = Level::grade(audio.drift_ms.abs(), DRIFT_WARN_MS, DRIFT_BAD_MS);
    let offset = Level::grade(audio.start_offset_ms.abs(), OFFSET_WARN_MS, OFFSET_BAD_MS);
    gap.max(drift).max(offset)
}

fn finding(area: Area, level: Level, text: String) -> Finding {
    Finding { area, level, text }
}

pub(super) fn findings(health: &RecordingHealth) -> Vec<Finding> {
    let mut out = Vec::new();
    let dropped = Level::grade(health.dropped_pct, DROPPED_WARN_PCT, DROPPED_BAD_PCT);
    out.push(finding(
        Area::Video,
        dropped,
        format!(
            "{:.1} of {} fps, {:.1}% dropped ({} of {} frames)",
            health.effective_fps(),
            health.target_fps,
            health.dropped_pct,
            health.frames_dropped,
            health.frames_expected
        ),
    ));
    let repeated = Level::grade(health.repeated_pct, REPEATED_WARN_PCT, REPEATED_BAD_PCT);
    if repeated > Level::Ok {
        out.push(finding(
            Area::Pacing,
            repeated,
            format!(
                "{:.1}% repeated frames: the game ran at about {:.0} fps",
                health.repeated_pct, health.source_fps
            ),
        ));
    }
    let gap = Level::grade(health.longest_video_gap_ms, GAP_WARN_MS, GAP_BAD_MS);
    out.push(finding(
        Area::Gaps,
        gap,
        format!("longest video gap {:.0} ms", health.longest_video_gap_ms),
    ));
    if let Some(timing) = health.encode_ms {
        let frame_ms = 1000.0 / f64::from(health.target_fps.max(1));
        let level = if timing.p95 > frame_ms * 2.0 {
            Level::Bad
        } else if timing.p95 > frame_ms {
            Level::Warn
        } else {
            Level::Ok
        };
        out.push(finding(
            Area::Encoder,
            level,
            format!(
                "encode latency avg {:.1} ms, p95 {:.1} ms, max {:.1} ms",
                timing.avg, timing.p95, timing.max
            ),
        ));
    }
    match &health.audio {
        Some(audio) => out.push(finding(
            Area::Audio,
            audio_level(audio),
            format!(
                "{} gaps (longest {:.0} ms), drift {:+.0} ms, start offset {:+.0} ms",
                audio.gaps, audio.longest_gap_ms, audio.drift_ms, audio.start_offset_ms
            ),
        )),
        None => out.push(finding(Area::Audio, Level::Ok, "no audio track".to_owned())),
    }
    if let Some(secs) = health.video_stopped_secs {
        out.push(finding(
            Area::File,
            Level::Bad,
            format!("the game video stopped after {secs:.0} s: the disk write failed"),
        ));
    }
    if health.capture_errors > 0 || health.capture_soft_errors > 0 {
        let level = if health.capture_errors > 0 {
            Level::Bad
        } else {
            Level::Warn
        };
        out.push(finding(
            Area::Capture,
            level,
            format!(
                "{} capture errors, {} recovered glitches",
                health.capture_errors, health.capture_soft_errors
            ),
        ));
    }
    if health.frames_torn_dropped > 0 || health.copy_wait_timeouts > 0 {
        let level = if health.copy_wait_timeouts > 0 {
            Level::Warn
        } else {
            Level::Ok
        };
        out.push(finding(
            Area::Capture,
            level,
            format!(
                "{} torn frames from Windows capture replaced, {} late and {} slow GPU copies",
                health.frames_torn_dropped, health.copy_wait_timeouts, health.copy_wait_slow
            ),
        ));
    }
    if health.frame_stalls > 0 {
        let level = if health.longest_frame_stall_secs >= STALL_BAD_SECS {
            Level::Bad
        } else {
            Level::Warn
        };
        out.push(finding(
            Area::Capture,
            level,
            format!(
                "{} stalls with no new game frame while audio ran (longest {:.0} s); the last frame was held",
                health.frame_stalls, health.longest_frame_stall_secs
            ),
        ));
    }
    let bitrate = if health.avg_bitrate_kbps < MIN_BITRATE_KBPS
        && health.height >= 720
        && health.duration_secs > 30.0
    {
        Level::Warn
    } else {
        Level::Ok
    };
    out.push(finding(
        Area::File,
        bitrate,
        format!(
            "{:.1} MB, average {:.1} Mbps",
            health.file_bytes as f64 / 1_000_000.0,
            f64::from(health.avg_bitrate_kbps) / 1000.0
        ),
    ));
    out
}
