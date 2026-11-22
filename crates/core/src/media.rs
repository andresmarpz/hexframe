use std::sync::Arc;

use crossbeam_channel::Sender;

use crate::time::{HNS_PER_SEC, Hns};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub track: Track,
    pub dts: i64,
    pub pts: i64,
    pub duration: u32,
    pub key: bool,
    pub data: Arc<[u8]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFormat {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub sps: Vec<u8>,
    pub pps: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bitrate: u32,
    pub asc: Vec<u8>,
}

impl VideoFormat {
    pub const TIMESCALE: u32 = 90_000;

    pub fn frame_ticks(&self) -> u32 {
        Self::TIMESCALE / self.fps
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureTarget {
    GameWindow { hwnd: isize, pid: u32 },
    PrimaryMonitor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureConfig {
    pub fps: u32,
    pub max_height: Option<u32>,
    pub video_bitrate: u32,
    pub video_peak: u32,
    pub quality: Option<u32>,
    pub gop_frames: u32,
    pub audio: bool,
    pub audio_bitrate: u32,
    pub cursor: bool,
    pub bitrate_pixels: u64,
    pub diagnostics: bool,
}

impl CaptureConfig {
    pub fn for_encode_size(&self, width: u32, height: u32) -> Self {
        let pixels = u64::from(width) * u64::from(height);
        if self.bitrate_pixels == 0 || pixels >= self.bitrate_pixels {
            return self.clone();
        }
        let ratio = (pixels as f64 / self.bitrate_pixels as f64).powf(0.75);
        let scale = |bits: u32| (f64::from(bits) * ratio).round() as u32;
        Self {
            video_bitrate: scale(self.video_bitrate),
            video_peak: scale(self.video_peak),
            bitrate_pixels: pixels,
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaEvent {
    Started {
        t0: Hns,
        video: VideoFormat,
        audio: Option<AudioFormat>,
        encoder: String,
    },
    Packet(Packet),
    Stats(Box<crate::health::CaptureStats>),
    Setup(Box<crate::health::CaptureSetup>),
    Warning(String),
    Failed(String),
}

pub trait MediaSource: Send + Sync {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String>;

    fn encoder_fingerprint(&self) -> Option<String> {
        None
    }

    fn check_encoders(&self, _cfg: &CaptureConfig, _every: bool) -> Vec<crate::encoder::Attempt> {
        Vec::new()
    }

    fn encoder_probes(&self) -> Vec<crate::encoder::Probe> {
        Vec::new()
    }

    fn probe_encoder(
        &self,
        _cfg: &CaptureConfig,
        _probe: &crate::encoder::Probe,
    ) -> Result<String, String> {
        Err("this source has no encoder probe".into())
    }
}

pub trait MediaSession: Send {
    fn request_keyframe(&self);
    fn stop(self: Box<Self>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Silence(usize),
    Data { skip: usize, take: usize },
}

#[derive(Debug, Clone)]
pub struct AudioTimeline {
    t0: Hns,
    rate: u32,
    frames_out: u64,
    trust_device_clock: bool,
    seen_first: bool,
    big_gaps: u32,
    largest_gap: Hns,
}

impl AudioTimeline {
    pub(crate) const RESYNC: Hns = 300_000;
    pub(crate) const FIRST_DISTRUST: Hns = 5_000_000;
    pub(crate) const TIMEOUT_SAFETY: Hns = 600_000;
    pub const BIG_GAP: Hns = 50_000_000;
    pub const MAX_SILENCE_CHUNK: Hns = HNS_PER_SEC;

    pub fn new(t0: Hns, rate: u32) -> Option<Self> {
        (rate > 0).then_some(Self {
            t0,
            rate,
            frames_out: 0,
            trust_device_clock: true,
            seen_first: false,
            big_gaps: 0,
            largest_gap: 0,
        })
    }

    pub fn big_gaps(&self) -> u32 {
        self.big_gaps
    }

    pub fn largest_gap(&self) -> Hns {
        self.largest_gap
    }

    #[cfg(test)]
    pub(crate) fn with_frames_out(mut self, frames_out: u64) -> Self {
        self.frames_out = frames_out;
        self
    }

    fn note_gap(&mut self, gap: Hns) {
        let size = gap.saturating_abs();
        if size > Self::BIG_GAP {
            self.big_gaps = self.big_gaps.saturating_add(1);
        }
        self.largest_gap = self.largest_gap.max(size);
    }

    fn push_silence(&self, spans: &mut Vec<Span>, mut frames: usize) {
        let chunk = self.frames_for_hns(Self::MAX_SILENCE_CHUNK).max(1);
        while frames > 0 {
            let n = frames.min(chunk);
            spans.push(Span::Silence(n));
            frames -= n;
        }
    }

    pub fn frames_out(&self) -> u64 {
        self.frames_out
    }

    pub fn on_packet_now(
        &mut self,
        ts: Option<Hns>,
        now: Hns,
        frames: usize,
        silent: bool,
    ) -> Vec<Span> {
        if !self.seen_first
            && let Some(first) = ts
            && first.saturating_sub(now).saturating_abs() > Self::FIRST_DISTRUST
        {
            self.trust_device_clock = false;
        }
        if !self.trust_device_clock {
            let span = self.hns_for_frames(frames as u64);
            return self.place(Some(now.saturating_sub(span)), frames, silent);
        }
        self.on_packet(ts, frames, silent)
    }

    pub(crate) fn on_packet(&mut self, ts: Option<Hns>, frames: usize, silent: bool) -> Vec<Span> {
        let ts = if self.trust_device_clock { ts } else { None };
        self.place(ts, frames, silent)
    }

    fn place(&mut self, ts: Option<Hns>, frames: usize, silent: bool) -> Vec<Span> {
        self.seen_first = true;
        let mut spans = Vec::new();
        let mut skip = 0usize;
        if let Some(ts) = ts {
            let gap = ts.saturating_sub(self.expected());
            self.note_gap(gap);
            if gap > Self::RESYNC {
                self.push_silence(&mut spans, self.frames_for_hns(gap));
            } else if gap < -Self::RESYNC {
                skip = self.frames_for_hns(gap.saturating_neg()).min(frames);
            }
        }
        let take = frames - skip;
        if take > 0 {
            spans.push(if silent {
                Span::Silence(take)
            } else {
                Span::Data { skip, take }
            });
        }
        self.frames_out += spans.iter().map(Self::span_frames).sum::<u64>();
        spans
    }

    pub fn on_timeout(&mut self, now: Hns) -> Option<Span> {
        let gap = now
            .saturating_sub(Self::TIMEOUT_SAFETY)
            .saturating_sub(self.expected());
        if gap <= 0 {
            return None;
        }
        let frames = self.frames_for_hns(gap);
        if frames == 0 {
            return None;
        }
        self.frames_out += frames as u64;
        Some(Span::Silence(frames))
    }

    fn expected(&self) -> Hns {
        self.t0.saturating_add(self.hns_for_frames(self.frames_out))
    }

    fn hns_for_frames(&self, frames: u64) -> Hns {
        let hns = i128::from(frames) * i128::from(HNS_PER_SEC) / i128::from(self.rate);
        Hns::try_from(hns).unwrap_or(Hns::MAX)
    }

    fn frames_for_hns(&self, hns: Hns) -> usize {
        let frames = i128::from(hns.max(0)) * i128::from(self.rate) / i128::from(HNS_PER_SEC);
        usize::try_from(frames).unwrap_or(usize::MAX)
    }

    fn span_frames(span: &Span) -> u64 {
        match *span {
            Span::Silence(n) => n as u64,
            Span::Data { take, .. } => take as u64,
        }
    }
}

pub fn encode_size(native: (u32, u32), max_height: Option<u32>) -> (u32, u32) {
    let (width, height) = native;
    let (width, height) = match max_height {
        Some(max) if height > max && height > 0 => {
            let scaled = u64::from(width) * u64::from(max) / u64::from(height);
            (scaled as u32, max)
        }
        _ => (width, height),
    };
    (width & !1, height & !1)
}

const REFRESH_MARGIN_HNS: i64 = 5_000;

pub fn capture_min_update_interval_hns(fps: u32, display_hz: u32) -> i64 {
    let fps = i64::from(fps.max(1));
    let cap = HNS_PER_SEC * 9 / 10 / fps;
    if display_hz == 0 {
        return cap;
    }
    let refresh = HNS_PER_SEC / i64::from(display_hz);
    (HNS_PER_SEC / fps - refresh - REFRESH_MARGIN_HNS).clamp(0, cap)
}

#[cfg(test)]
mod reference;
#[cfg(test)]
mod tests;
