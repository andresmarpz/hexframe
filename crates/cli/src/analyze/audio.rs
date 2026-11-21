use greyscreen_mp4::moov::SampleRow;
use serde::Serialize;

const AAC_FRAME: u32 = 1024;
const MAX_LISTED: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AudioGap {
    pub at_secs: f64,
    pub length_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AudioReport {
    pub sample_rate: u32,
    pub packets: u64,
    pub samples: u64,
    pub timeline_secs: f64,
    pub coverage_pct: f64,
    pub gap_count: usize,
    pub longest_gap_ms: f64,
    pub gaps: Vec<AudioGap>,
    pub av_diff_ms: f64,
}

pub(crate) fn analyze(rows: &[SampleRow], sample_rate: u32, video_secs: f64) -> AudioReport {
    let rate = f64::from(sample_rate.max(1));
    let mut gaps = Vec::new();
    let mut at = 0u64;
    for row in rows {
        if f64::from(row.duration) > f64::from(AAC_FRAME) * 1.5 {
            gaps.push(AudioGap {
                at_secs: at as f64 / rate,
                length_ms: f64::from(row.duration - AAC_FRAME) * 1000.0 / rate,
            });
        }
        at += u64::from(row.duration);
    }
    let packets = rows.len() as u64;
    let samples = packets * u64::from(AAC_FRAME);
    let timeline_secs = at as f64 / rate;
    let gap_count = gaps.len();
    let longest_gap_ms = gaps.iter().map(|g| g.length_ms).fold(0.0, f64::max);
    gaps.sort_by(|a, b| b.length_ms.total_cmp(&a.length_ms));
    gaps.truncate(MAX_LISTED);
    gaps.sort_by(|a, b| a.at_secs.total_cmp(&b.at_secs));
    AudioReport {
        sample_rate,
        packets,
        samples,
        timeline_secs,
        coverage_pct: if timeline_secs > 0.0 {
            samples as f64 / rate * 100.0 / timeline_secs
        } else {
            0.0
        },
        gap_count,
        longest_gap_ms,
        gaps,
        av_diff_ms: (timeline_secs - video_secs) * 1000.0,
    }
}
