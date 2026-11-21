use std::collections::VecDeque;
use std::sync::Arc;

use crossbeam_channel::Sender;
use greyscreen_core::health::{CaptureSetup, CaptureStats, LatencyHistogram};
use greyscreen_core::media::{AudioFormat, MediaEvent, Packet, Track, VideoFormat};
use greyscreen_core::time::{Hns, mono_now};
use greyscreen_mp4::avc::annexb_to_avcc;

use super::matching::{match_tick, parameter_fault};
use crate::h264::EncodedVideo;

const MAX_TRACKED_SUBMITS: usize = 256;

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
    in_flight: VecDeque<(u64, Hns)>,
    submit_times: VecDeque<(u64, Hns)>,
    encode: LatencyHistogram,
    window_encode_max_us: u32,
    last_tick: Option<u64>,
    frame_ticks: i64,
    parameters: Option<(Vec<u8>, Vec<u8>)>,
    fallback: Option<(Vec<u8>, Vec<u8>)>,
    in_band: bool,
    outputs_without_parameters: u32,
    fault: Option<String>,
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
            in_flight: VecDeque::new(),
            submit_times: VecDeque::new(),
            encode: LatencyHistogram::default(),
            window_encode_max_us: 0,
            last_tick: None,
            frame_ticks,
            parameters: None,
            fallback: None,
            in_band: false,
            outputs_without_parameters: 0,
            fault: None,
        }
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed
    }

    pub(crate) fn encoded(&self) -> u64 {
        self.out_index
    }

    pub(crate) fn take_fault(&mut self) -> Option<String> {
        self.fault.take()
    }

    pub(crate) fn submitted(&mut self, tick: u64, pts: Hns) {
        self.in_flight.push_back((tick, pts));
        self.submit_times.push_back((tick, mono_now()));
        if self.submit_times.len() > MAX_TRACKED_SUBMITS {
            self.submit_times.pop_front();
        }
    }

    fn record_latency(&mut self, tick: u64, now: Hns) {
        while self.submit_times.front().is_some_and(|&(t, _)| t < tick) {
            self.submit_times.pop_front();
        }
        if let Some(&(t, at)) = self.submit_times.front()
            && t == tick
        {
            self.submit_times.pop_front();
            let us = u32::try_from((now - at).max(0) / 10).unwrap_or(u32::MAX);
            self.encode.record(us);
            self.window_encode_max_us = self.window_encode_max_us.max(us);
        }
    }

    pub(crate) fn fill_stats(&mut self, stats: &mut CaptureStats) {
        stats.encode = self.encode.clone();
        stats.window_encode_max_us = std::mem::take(&mut self.window_encode_max_us);
        stats.in_flight = u32::try_from(self.in_flight.len()).unwrap_or(u32::MAX);
    }

    pub(crate) fn stats(&mut self, stats: Box<CaptureStats>) {
        if self.started {
            self.send(MediaEvent::Stats(stats));
        }
    }

    pub(crate) fn setup(&mut self, setup: CaptureSetup) {
        self.queue(MediaEvent::Setup(Box::new(setup)));
    }

    fn next_tick(&mut self, output_pts: Hns) -> u64 {
        let fallback = self.last_tick.map_or(0, |last| last + 1);
        let matched = match_tick(&mut self.in_flight, output_pts, self.out_index == 0);
        let tick = matched.map_or(fallback, |tick| tick.max(fallback));
        self.last_tick = Some(tick);
        tick
    }

    pub(crate) fn set_fallback(&mut self, sps: Vec<u8>, pps: Vec<u8>) {
        self.fallback = Some((sps, pps));
    }

    pub(crate) fn start(&mut self, sps: Vec<u8>, pps: Vec<u8>) {
        if self.started {
            return;
        }
        self.started = true;
        self.parameters = Some((sps.clone(), pps.clone()));
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
        let now = mono_now();
        for item in encoded.drain(..) {
            if self.fault.is_some() {
                continue;
            }
            let (sample, info) = annexb_to_avcc(&item.annexb);
            if !self.started {
                if let (Some(sps), Some(pps)) = (&info.sps, &info.pps) {
                    self.in_band = true;
                    self.start(sps.clone(), pps.clone());
                } else if let Some((sps, pps)) = self.fallback.take() {
                    self.start(sps, pps);
                } else {
                    self.outputs_without_parameters += 1;
                }
            }
            // The encoder's header copy can differ in bytes from its in-band sets, so only in-band starts are compared.
            let known = self
                .parameters
                .as_ref()
                .filter(|_| self.in_band)
                .map(|(sps, pps)| (sps.as_slice(), pps.as_slice()));
            let fault = if self.started && !self.in_band {
                None
            } else {
                parameter_fault(
                    known,
                    info.sps.as_deref(),
                    info.pps.as_deref(),
                    self.outputs_without_parameters,
                )
            };
            if let Some(fault) = fault {
                self.fault = Some(fault.to_string());
                continue;
            }
            if sample.is_empty() {
                continue;
            }
            let tick = self.next_tick(item.pts_hns);
            self.record_latency(tick, now);
            let ticks = tick as i64 * self.frame_ticks;
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
