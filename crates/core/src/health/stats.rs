use serde::Serialize;

use crate::time::Hns;

const BUCKETS: usize = 100;
const BUCKET_US: u32 = 500;

#[derive(Debug, Clone, PartialEq)]
pub struct LatencyHistogram {
    buckets: [u32; BUCKETS],
    count: u64,
    sum_us: u64,
    max_us: u32,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self {
            buckets: [0; BUCKETS],
            count: 0,
            sum_us: 0,
            max_us: 0,
        }
    }
}

impl LatencyHistogram {
    pub fn record(&mut self, us: u32) {
        let index = ((us / BUCKET_US) as usize).min(BUCKETS - 1);
        self.buckets[index] = self.buckets[index].saturating_add(1);
        self.count += 1;
        self.sum_us += u64::from(us);
        self.max_us = self.max_us.max(us);
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn avg_ms(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum_us as f64 / self.count as f64 / 1000.0
    }

    pub fn max_ms(&self) -> f64 {
        f64::from(self.max_us) / 1000.0
    }

    pub fn percentile_ms(&self, p: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let target = (self.count as f64 * p).ceil().max(1.0) as u64;
        let mut seen = 0u64;
        for (index, &n) in self.buckets.iter().enumerate() {
            seen += u64::from(n);
            if seen >= target {
                let upper = (index as u32 + 1) * BUCKET_US;
                return f64::from(upper.min(self.max_us.max(1))) / 1000.0;
            }
        }
        self.max_ms()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CaptureStats {
    pub at: Hns,
    pub ticks: u64,
    pub captured: u64,
    pub repeated: u64,
    pub dropped: u64,
    pub skipped: u64,
    pub resyncs: u64,
    pub soft_errors: u64,
    pub encode: LatencyHistogram,
    pub window_encode_max_us: u32,
    pub window_step_max_us: u32,
    pub audio_packets: u64,
    pub audio_discontinuities: u64,
    pub audio_gaps: u64,
    pub audio_gap_longest_ms: u32,
    pub audio_gap_total_ms: u64,
    pub audio_idle_ms: u64,
    pub in_flight: u32,
    pub pcm_queue: u32,
    pub cpu_pct: Option<f32>,
    pub mem_mb: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct CaptureSetup {
    pub adapter: String,
    pub driver_version: String,
    pub display_width: u32,
    pub display_height: u32,
    pub display_hz: u32,
    pub window_width: u32,
    pub window_height: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub crop_left: u32,
    pub crop_top: u32,
    pub crop_width: u32,
    pub crop_height: u32,
    pub encode_width: u32,
    pub encode_height: u32,
    pub fps: u32,
    pub encoder: String,
    pub bitrate: u32,
    pub peak: u32,
    pub quality: Option<u32>,
    pub gop_frames: u32,
    pub audio: bool,
}
