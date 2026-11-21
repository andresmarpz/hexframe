use greyscreen_core::media::VideoFormat;
use greyscreen_mp4::moov::SampleRow;
use serde::Serialize;

const MAX_LISTED: usize = 20;
const GAP_FACTOR: f64 = 2.0;
const WINDOW_SECS: u64 = 10;
const LABELS: [&str; 6] = [
    "1 frame",
    "2 frames",
    "3 frames",
    "4-5 frames",
    "6-10 frames",
    "over 10 frames",
];

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Bucket {
    pub label: &'static str,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Gap {
    pub at_secs: f64,
    pub length_ms: f64,
    pub missing_frames: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Gaps {
    pub count: usize,
    pub longest_ms: f64,
    pub listed: Vec<Gap>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Keyframes {
    pub count: usize,
    pub min_secs: f64,
    pub avg_secs: f64,
    pub max_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Bitrate {
    pub window_secs: u64,
    pub avg_kbps: u32,
    pub min_kbps: u32,
    pub max_kbps: u32,
    pub windows_kbps: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VideoReport {
    pub frames: u64,
    pub nominal_fps: u32,
    pub duration_secs: f64,
    pub average_fps: f64,
    pub missing_frames: u64,
    pub missing_pct: f64,
    pub histogram: Vec<Bucket>,
    pub gaps: Gaps,
    pub keyframes: Keyframes,
    pub bitrate: Bitrate,
}

pub(crate) struct AudioBytes<'a> {
    pub rows: &'a [SampleRow],
    pub sample_rate: u32,
}

pub(crate) fn analyze(rows: &[SampleRow], fps: u32, audio: Option<AudioBytes>) -> VideoReport {
    let timescale = u64::from(VideoFormat::TIMESCALE);
    let nominal = nominal_ticks(rows, fps);
    let mut histogram: Vec<Bucket> = LABELS
        .iter()
        .map(|&label| Bucket { label, count: 0 })
        .collect();
    let mut gaps = Vec::new();
    let mut missing = 0u64;
    let mut keys = Vec::new();
    let mut at = 0u64;
    for row in rows {
        let ratio = (f64::from(row.duration) / nominal as f64).round().max(1.0) as u64;
        histogram[bucket(ratio)].count += 1;
        if f64::from(row.duration) > nominal as f64 * GAP_FACTOR {
            gaps.push(Gap {
                at_secs: (at + nominal) as f64 / timescale as f64,
                length_ms: f64::from(row.duration) * 1000.0 / timescale as f64,
                missing_frames: ratio - 1,
            });
        }
        if ratio > 1 {
            missing += ratio - 1;
        }
        if row.sync {
            keys.push(at as f64 / timescale as f64);
        }
        at += u64::from(row.duration);
    }
    let duration_secs = at as f64 / timescale as f64;
    let frames = rows.len() as u64;
    VideoReport {
        frames,
        nominal_fps: if nominal > 0 {
            (timescale / nominal) as u32
        } else {
            0
        },
        duration_secs,
        average_fps: if duration_secs > 0.0 {
            frames as f64 / duration_secs
        } else {
            0.0
        },
        missing_frames: missing,
        missing_pct: if frames + missing > 0 {
            missing as f64 * 100.0 / (frames + missing) as f64
        } else {
            0.0
        },
        histogram,
        gaps: summarize_gaps(gaps),
        keyframes: keyframes(&keys),
        bitrate: bitrate(rows, audio, duration_secs),
    }
}

fn nominal_ticks(rows: &[SampleRow], fps: u32) -> u64 {
    if fps > 0 {
        return u64::from(VideoFormat::TIMESCALE / fps);
    }
    let mut durations: Vec<u32> = rows.iter().map(|r| r.duration).collect();
    durations.sort_unstable();
    u64::from(durations.get(durations.len() / 2).copied().unwrap_or(1500)).max(1)
}

fn bucket(ratio: u64) -> usize {
    match ratio {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4..=5 => 3,
        6..=10 => 4,
        _ => 5,
    }
}

fn summarize_gaps(all: Vec<Gap>) -> Gaps {
    let count = all.len();
    let longest_ms = all.iter().map(|g| g.length_ms).fold(0.0, f64::max);
    let mut listed = all;
    listed.sort_by(|a, b| b.length_ms.total_cmp(&a.length_ms));
    listed.truncate(MAX_LISTED);
    listed.sort_by(|a, b| a.at_secs.total_cmp(&b.at_secs));
    Gaps {
        count,
        longest_ms,
        listed,
    }
}

fn keyframes(times: &[f64]) -> Keyframes {
    let intervals: Vec<f64> = times.windows(2).map(|w| w[1] - w[0]).collect();
    let min = intervals.iter().copied().fold(f64::INFINITY, f64::min);
    let max = intervals.iter().copied().fold(0.0, f64::max);
    let avg = if intervals.is_empty() {
        0.0
    } else {
        intervals.iter().sum::<f64>() / intervals.len() as f64
    };
    Keyframes {
        count: times.len(),
        min_secs: if min.is_finite() { min } else { 0.0 },
        avg_secs: avg,
        max_secs: max,
    }
}

fn add_bytes(windows: &mut Vec<u64>, rows: &[SampleRow], timescale: u32) {
    let span = u64::from(timescale) * WINDOW_SECS;
    let mut at = 0u64;
    for row in rows {
        let index = (at / span) as usize;
        if windows.len() <= index {
            windows.resize(index + 1, 0);
        }
        windows[index] += u64::from(row.size);
        at += u64::from(row.duration);
    }
}

fn bitrate(rows: &[SampleRow], audio: Option<AudioBytes>, duration_secs: f64) -> Bitrate {
    let mut windows = Vec::new();
    add_bytes(&mut windows, rows, VideoFormat::TIMESCALE);
    if let Some(audio) = audio {
        add_bytes(&mut windows, audio.rows, audio.sample_rate);
    }
    let last = windows.len().saturating_sub(1);
    let kbps: Vec<u32> = windows
        .iter()
        .enumerate()
        .map(|(index, &bytes)| {
            let secs = if index == last {
                (duration_secs - last as f64 * WINDOW_SECS as f64).clamp(1.0, WINDOW_SECS as f64)
            } else {
                WINDOW_SECS as f64
            };
            (bytes as f64 * 8.0 / secs / 1000.0).round() as u32
        })
        .collect();
    let total: u64 = windows.iter().sum();
    Bitrate {
        window_secs: WINDOW_SECS,
        avg_kbps: if duration_secs > 0.0 {
            (total as f64 * 8.0 / duration_secs / 1000.0).round() as u32
        } else {
            0
        },
        min_kbps: kbps.iter().copied().min().unwrap_or(0),
        max_kbps: kbps.iter().copied().max().unwrap_or(0),
        windows_kbps: kbps,
    }
}
