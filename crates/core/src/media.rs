use std::sync::Arc;

use crossbeam_channel::Sender;

use crate::time::Hns;

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
    pub gop_frames: u32,
    pub audio: bool,
    pub audio_bitrate: u32,
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
}

pub trait MediaSession: Send {
    fn request_keyframe(&self);
    fn stop(self: Box<Self>);
}

#[cfg(test)]
mod tests;
