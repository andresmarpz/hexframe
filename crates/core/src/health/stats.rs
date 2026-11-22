use serde::Serialize;

use crate::time::Hns;

const BUCKETS: usize = 100;
const BUCKET_US: u32 = 500;
const FINE_BUCKET_US: u32 = 50;

#[derive(Debug, Clone, PartialEq)]
pub struct LatencyHistogram {
    buckets: [u32; BUCKETS],
    bucket_us: u32,
    count: u64,
    sum_us: u64,
    max_us: u32,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self {
            buckets: [0; BUCKETS],
            bucket_us: BUCKET_US,
            count: 0,
            sum_us: 0,
            max_us: 0,
        }
    }
}

impl LatencyHistogram {
    pub fn fine() -> Self {
        Self {
            bucket_us: FINE_BUCKET_US,
            ..Self::default()
        }
    }

    pub fn record(&mut self, us: u32) {
        let index = ((us / self.bucket_us) as usize).min(BUCKETS - 1);
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

    pub fn percentile_us(&self, p: f64) -> u32 {
        (self.percentile_ms(p) * 1000.0).round() as u32
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
                let upper = (index as u32 + 1) * self.bucket_us;
                return f64::from(upper.min(self.max_us.max(1))) / 1000.0;
            }
        }
        self.max_ms()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WakeLateness(pub LatencyHistogram);

impl Default for WakeLateness {
    fn default() -> Self {
        Self(LatencyHistogram::fine())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpinShare {
    spin_us: u64,
    wall_us: u64,
}

impl SpinShare {
    pub fn add(&mut self, spin_us: u64, wall_us: u64) {
        self.spin_us = self.spin_us.saturating_add(spin_us);
        self.wall_us = self.wall_us.saturating_add(wall_us.max(spin_us));
    }

    pub fn pct(&self) -> f32 {
        if self.wall_us == 0 {
            return 0.0;
        }
        (self.spin_us as f64 * 100.0 / self.wall_us as f64) as f32
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
    pub torn_dropped: u64,
    pub copy_wait_slow: u64,
    pub copy_wait_timeouts: u64,
    pub window_copy_wait_max_us: u32,
    pub encode: LatencyHistogram,
    pub window_encode_max_us: u32,
    pub window_step_max_us: u32,
    pub wake_lateness: WakeLateness,
    pub pacer_spin: SpinShare,
    pub first_output_ms: Option<u32>,
    pub encoder_stalls_early: u32,
    pub audio_packets: u64,
    pub audio_discontinuities: u64,
    pub audio_gaps: u64,
    pub audio_gap_longest_ms: u32,
    pub audio_gap_total_ms: u64,
    pub audio_idle_ms: u64,
    pub audio_timestamp_gaps_over_5s: u32,
    pub audio_largest_timestamp_gap_ms: u64,
    pub in_flight: u32,
    pub pcm_queue: u32,
    pub cpu_pct: Option<f32>,
    pub mem_mb: Option<u32>,
    pub foreground: Option<bool>,
    pub gpu_3d_pct: Option<f32>,
    pub gpu_3d_self_pct: Option<f32>,
    pub gpu_encode_pct: Option<f32>,
    pub gpu_copy_pct: Option<f32>,
}

const NVIDIA_VENDOR: u32 = 0x10DE;

pub fn driver_version_text(version: i64, vendor_id: u32) -> String {
    let v = version as u64;
    let parts = [
        (v >> 48) & 0xFFFF,
        (v >> 32) & 0xFFFF,
        (v >> 16) & 0xFFFF,
        v & 0xFFFF,
    ];
    let raw = format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);
    if vendor_id == NVIDIA_VENDOR {
        let (c, d) = (parts[2], parts[3]);
        return format!("{raw} (NVIDIA {}{}.{:02})", c % 10, d / 100, d % 100);
    }
    raw
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
    pub warnings: Vec<String>,
}
