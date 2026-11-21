use std::sync::Arc;

use crossbeam_channel::Sender;
use greyscreen_core::media::{AudioFormat, MediaEvent, Packet, Track, VideoFormat};
use greyscreen_core::time::Hns;
use greyscreen_mp4::avc::annexb_to_avcc;

use crate::h264::EncodedVideo;

#[derive(Debug, Clone)]
pub(crate) struct StartInfo {
    pub(crate) t0: Hns,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) fps: u32,
    pub(crate) audio: Option<AudioFormat>,
    pub(crate) encoder: String,
}

#[derive(Debug)]
pub(crate) struct Sink {
    out: Sender<MediaEvent>,
    info: StartInfo,
    started: bool,
    closed: bool,
    pending: Vec<MediaEvent>,
    out_index: u64,
    frame_ticks: i64,
}

impl Sink {
    pub(crate) fn new(out: Sender<MediaEvent>, info: StartInfo) -> Self {
        let frame_ticks = i64::from(VideoFormat::TIMESCALE / info.fps.max(1));
        Self {
            out,
            info,
            started: false,
            closed: false,
            pending: Vec::new(),
            out_index: 0,
            frame_ticks,
        }
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed
    }

    pub(crate) fn encoded(&self) -> u64 {
        self.out_index
    }

    pub(crate) fn start(&mut self, sps: Vec<u8>, pps: Vec<u8>) {
        if self.started {
            return;
        }
        self.started = true;
        let event = MediaEvent::Started {
            t0: self.info.t0,
            video: VideoFormat {
                width: self.info.width,
                height: self.info.height,
                fps: self.info.fps,
                sps,
                pps,
            },
            audio: self.info.audio.clone(),
            encoder: self.info.encoder.clone(),
        };
        self.send(event);
        for event in std::mem::take(&mut self.pending) {
            self.send(event);
        }
    }

    pub(crate) fn video(&mut self, encoded: &mut Vec<EncodedVideo>) {
        for item in encoded.drain(..) {
            let (sample, info) = annexb_to_avcc(&item.annexb);
            if !self.started
                && let (Some(sps), Some(pps)) = (info.sps, info.pps)
            {
                self.start(sps, pps);
            }
            if sample.is_empty() {
                continue;
            }
            let ticks = self.out_index as i64 * self.frame_ticks;
            self.out_index += 1;
            self.packet(Packet {
                track: Track::Video,
                dts: ticks,
                pts: ticks,
                duration: self.frame_ticks as u32,
                key: info.idr,
                data: Arc::from(sample),
            });
        }
    }

    pub(crate) fn packet(&mut self, packet: Packet) {
        self.queue(MediaEvent::Packet(packet));
    }

    pub(crate) fn warning(&mut self, message: String) {
        self.queue(MediaEvent::Warning(message));
    }

    pub(crate) fn failed(&mut self, reason: String) {
        self.send(MediaEvent::Failed(reason));
    }

    fn queue(&mut self, event: MediaEvent) {
        if self.started {
            self.send(event);
        } else {
            self.pending.push(event);
        }
    }

    fn send(&mut self, event: MediaEvent) {
        if self.out.send(event).is_err() {
            self.closed = true;
        }
    }
}
