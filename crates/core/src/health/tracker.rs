use serde::Serialize;

use super::{AudioHealth, CaptureStats, EncodeTiming, RecordingHealth, hns_ms, ticks_to_ms};
use crate::media::{Packet, Track, VideoFormat};
use crate::time::Hns;

const AAC_FRAME_TICKS: i64 = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Queues {
    pub writer: u32,
    pub media: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiagSample {
    pub t: f64,
    pub captured: u64,
    pub encoded: u64,
    pub dropped: u64,
    pub repeated: u64,
    pub encode_ms_max: f64,
    pub step_ms_max: f64,
    pub torn: u64,
    pub copy_wait_ms_max: f64,
    pub audio_packets: u64,
    pub av_skew_ms: f64,
    pub queue_writer: u32,
    pub queue_media: u32,
    pub queue_encoder: u32,
    pub queue_pcm: u32,
    pub cpu_pct: Option<f32>,
    pub mem_mb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_3d_pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_3d_self_pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_encode_pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_copy_pct: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    first_dts: Option<i64>,
    last_dts: i64,
    end: i64,
    packets: u64,
    bytes: u64,
    longest_delta: i64,
}

impl Tally {
    fn add(&mut self, packet: &Packet) -> Option<i64> {
        let delta = self.first_dts.map(|_| packet.dts - self.last_dts);
        self.first_dts.get_or_insert(packet.dts);
        self.last_dts = packet.dts;
        self.end = packet.dts + i64::from(packet.duration);
        self.packets += 1;
        self.bytes += packet.data.len() as u64;
        if let Some(delta) = delta {
            self.longest_delta = self.longest_delta.max(delta);
        }
        delta
    }

    fn span(&self, from: i64) -> i64 {
        (self.end - from).max(0)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Marks {
    captured: u64,
    encoded: u64,
    dropped: u64,
    repeated: u64,
    torn: u64,
}

#[derive(Debug, Clone)]
pub struct HealthTracker {
    t0: Hns,
    fps: u32,
    width: u32,
    height: u32,
    encoder: String,
    audio_rate: Option<u32>,
    frame_ticks: i64,
    video: Tally,
    missing_frames: u64,
    audio: Tally,
    audio_gaps: u32,
    audio_longest_gap: i64,
    errors: u32,
    last: Option<CaptureStats>,
    marks: Marks,
}

impl HealthTracker {
    pub fn new(t0: Hns, video: &VideoFormat, encoder: &str, audio_rate: Option<u32>) -> Self {
        Self {
            t0,
            fps: video.fps,
            width: video.width,
            height: video.height,
            encoder: encoder.to_owned(),
            audio_rate,
            frame_ticks: i64::from(video.frame_ticks().max(1)),
            video: Tally::default(),
            missing_frames: 0,
            audio: Tally::default(),
            audio_gaps: 0,
            audio_longest_gap: 0,
            errors: 0,
            last: None,
            marks: Marks::default(),
        }
    }

    pub fn on_packet(&mut self, packet: &Packet) {
        match packet.track {
            Track::Video => {
                if let Some(delta) = self.video.add(packet)
                    && delta > self.frame_ticks
                {
                    self.missing_frames += ((delta - 1) / self.frame_ticks).max(1) as u64;
                }
            }
            Track::Audio => {
                if let Some(delta) = self.audio.add(packet)
                    && delta > AAC_FRAME_TICKS
                {
                    self.audio_gaps += 1;
                    self.audio_longest_gap = self.audio_longest_gap.max(delta - AAC_FRAME_TICKS);
                }
            }
        }
    }

    pub fn has_video(&self) -> bool {
        self.video.packets > 0
    }

    pub fn on_failure(&mut self) {
        self.errors += 1;
    }

    pub fn on_stats(&mut self, stats: &CaptureStats, queues: Queues) -> DiagSample {
        let marks = self.marks;
        let dropped = self.missing_frames;
        self.marks = Marks {
            captured: stats.captured,
            encoded: self.video.packets,
            dropped,
            repeated: stats.repeated,
            torn: stats.torn_dropped,
        };
        self.last = Some(stats.clone());
        DiagSample {
            t: round1(hns_ms(stats.at - self.t0) / 1000.0),
            captured: stats.captured.saturating_sub(marks.captured),
            encoded: self.video.packets.saturating_sub(marks.encoded),
            dropped: dropped.saturating_sub(marks.dropped),
            repeated: stats.repeated.saturating_sub(marks.repeated),
            encode_ms_max: round1(f64::from(stats.window_encode_max_us) / 1000.0),
            step_ms_max: round1(f64::from(stats.window_step_max_us) / 1000.0),
            torn: stats.torn_dropped.saturating_sub(marks.torn),
            copy_wait_ms_max: round1(f64::from(stats.window_copy_wait_max_us) / 1000.0),
            audio_packets: self.audio.packets,
            av_skew_ms: round1(self.av_skew_ms()),
            queue_writer: queues.writer,
            queue_media: queues.media,
            queue_encoder: stats.in_flight,
            queue_pcm: stats.pcm_queue,
            cpu_pct: stats.cpu_pct,
            mem_mb: stats.mem_mb,
            foreground: stats.foreground,
            gpu_3d_pct: stats.gpu_3d_pct,
            gpu_3d_self_pct: stats.gpu_3d_self_pct,
            gpu_encode_pct: stats.gpu_encode_pct,
            gpu_copy_pct: stats.gpu_copy_pct,
        }
    }

    fn av_skew_ms(&self) -> f64 {
        let Some(rate) = self.audio_rate else {
            return 0.0;
        };
        if self.audio.packets == 0 || self.video.packets == 0 {
            return 0.0;
        }
        ticks_to_ms(self.audio.end, rate) - ticks_to_ms(self.video.end, VideoFormat::TIMESCALE)
    }

    pub fn finish(&self, file_bytes: Option<u64>) -> RecordingHealth {
        let origin = self.video.first_dts.unwrap_or(0);
        let duration_secs = ticks_to_ms(self.video.span(origin), VideoFormat::TIMESCALE) / 1000.0;
        let encoded = self.video.packets;
        let expected = encoded + self.missing_frames;
        let stats = self.last.clone().unwrap_or_default();
        let bytes = file_bytes.unwrap_or(self.video.bytes + self.audio.bytes);
        let pct = |part: u64, whole: u64| {
            if whole == 0 {
                0.0
            } else {
                round2(part as f64 * 100.0 / whole as f64)
            }
        };
        RecordingHealth {
            target_fps: self.fps,
            width: self.width,
            height: self.height,
            duration_secs: round2(duration_secs),
            frames_expected: expected,
            frames_captured: stats.captured,
            frames_encoded: encoded,
            frames_dropped: self.missing_frames,
            frames_repeated: stats.repeated,
            dropped_pct: pct(self.missing_frames, expected),
            repeated_pct: pct(stats.repeated, encoded),
            source_fps: if duration_secs > 0.0 {
                round1(stats.captured as f64 / duration_secs)
            } else {
                0.0
            },
            longest_video_gap_ms: round1(ticks_to_ms(
                self.video.longest_delta,
                VideoFormat::TIMESCALE,
            )),
            encoder: self.encoder.clone(),
            encode_ms: (stats.encode.count() > 0).then(|| EncodeTiming {
                avg: round2(stats.encode.avg_ms()),
                p95: round2(stats.encode.percentile_ms(0.95)),
                max: round2(stats.encode.max_ms()),
            }),
            audio: self
                .audio_rate
                .map(|rate| self.audio_health(rate, origin, &stats)),
            file_bytes: bytes,
            avg_bitrate_kbps: if duration_secs > 0.0 {
                (bytes as f64 * 8.0 / duration_secs / 1000.0).round() as u32
            } else {
                0
            },
            capture_errors: self.errors,
            capture_soft_errors: stats.soft_errors,
            frames_torn_dropped: stats.torn_dropped,
            copy_wait_slow: stats.copy_wait_slow,
            copy_wait_timeouts: stats.copy_wait_timeouts,
        }
    }

    fn audio_health(&self, rate: u32, origin: i64, stats: &CaptureStats) -> AudioHealth {
        let origin_audio = origin * i64::from(rate) / i64::from(VideoFormat::TIMESCALE);
        let first = self.audio.first_dts.unwrap_or(origin_audio);
        let kept = if first < origin_audio {
            first + (origin_audio - first) / AAC_FRAME_TICKS * AAC_FRAME_TICKS
        } else {
            first
        };
        let video_secs = self.video.span(origin) as f64 / f64::from(VideoFormat::TIMESCALE);
        let audio_secs = self.audio.span(kept) as f64 / f64::from(rate);
        let has_audio = self.audio.packets > 0;
        let fill_gaps = u32::try_from(stats.audio_gaps).unwrap_or(u32::MAX);
        AudioHealth {
            packets: self.audio.packets,
            gaps: self.audio_gaps.saturating_add(fill_gaps),
            longest_gap_ms: round1(
                ticks_to_ms(self.audio_longest_gap, rate)
                    .max(f64::from(stats.audio_gap_longest_ms)),
            ),
            discontinuities: stats.audio_discontinuities,
            idle_silence_ms: stats.audio_idle_ms,
            start_offset_ms: if has_audio {
                round1(ticks_to_ms(kept - origin_audio, rate))
            } else {
                0.0
            },
            drift_ms: if has_audio {
                round1((audio_secs - video_secs) * 1000.0)
            } else {
                0.0
            },
        }
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
