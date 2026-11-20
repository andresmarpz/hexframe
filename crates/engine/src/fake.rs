use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, bounded, select};
use greyscreen_core::media::{
    AudioFormat, CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource, Packet,
    Track, VideoFormat,
};
use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now};
use greyscreen_mp4::aac::{asc_lc, split_adts};
use greyscreen_mp4::avc::{
    NAL_AUD, NAL_PPS, NAL_SEI, NAL_SPS, annexb_to_avcc, nal_type, parse_sps_dims, split_annexb,
};

const VIDEO_BYTES: &[u8] = include_bytes!("../fixtures/loop.h264");
const AUDIO_BYTES: &[u8] = include_bytes!("../fixtures/loop.aac");
const FPS: u32 = 60;
const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: u16 = 2;
const AUDIO_BITRATE: u32 = 192_000;
const AAC_FRAME_TICKS: u32 = 1024;
const IDLE_POLL: Duration = Duration::from_millis(5);

struct Frame {
    data: Arc<[u8]>,
    key: bool,
}

struct Fixtures {
    video: VideoFormat,
    audio: AudioFormat,
    video_frames: Vec<Frame>,
    audio_frames: Vec<Arc<[u8]>>,
}

fn fixtures() -> Result<&'static Fixtures, String> {
    static CELL: OnceLock<Result<Fixtures, String>> = OnceLock::new();
    CELL.get_or_init(parse_fixtures)
        .as_ref()
        .map_err(Clone::clone)
}

fn is_slice(nal: &[u8]) -> bool {
    matches!(nal_type(nal[0]), 1 | 5)
}

fn access_units(stream: &[u8]) -> Vec<Vec<u8>> {
    let mut units = Vec::new();
    let mut current = Vec::new();
    let mut has_slice = false;
    for nal in split_annexb(stream) {
        let kind = nal_type(nal[0]);
        let starts_unit = matches!(kind, NAL_AUD | NAL_SPS | NAL_PPS | NAL_SEI) || is_slice(nal);
        if has_slice && starts_unit {
            units.push(std::mem::take(&mut current));
            has_slice = false;
        }
        has_slice |= is_slice(nal);
        current.extend_from_slice(&[0, 0, 0, 1]);
        current.extend_from_slice(nal);
    }
    if has_slice {
        units.push(current);
    }
    units
}

fn parse_fixtures() -> Result<Fixtures, String> {
    let mut sps = None;
    let mut pps = None;
    let mut video_frames = Vec::new();
    for unit in access_units(VIDEO_BYTES) {
        let (data, info) = annexb_to_avcc(&unit);
        sps = sps.or(info.sps);
        pps = pps.or(info.pps);
        video_frames.push(Frame {
            data: data.into(),
            key: info.idr,
        });
    }
    let sps = sps.ok_or("video fixture has no SPS")?;
    let pps = pps.ok_or("video fixture has no PPS")?;
    let (width, height, _, _) = parse_sps_dims(&sps).ok_or("video fixture SPS does not parse")?;
    let audio_frames: Vec<Arc<[u8]>> = split_adts(AUDIO_BYTES).into_iter().map(Arc::from).collect();
    if video_frames.is_empty() || audio_frames.is_empty() {
        return Err("fixture has no frames".into());
    }
    Ok(Fixtures {
        video: VideoFormat {
            width,
            height,
            fps: FPS,
            sps,
            pps,
        },
        audio: AudioFormat {
            sample_rate: SAMPLE_RATE,
            channels: CHANNELS,
            bitrate: AUDIO_BITRATE,
            asc: asc_lc(SAMPLE_RATE, CHANNELS),
        },
        video_frames,
        audio_frames,
    })
}

#[derive(Debug, Clone, Copy)]
pub struct FakeMediaSource {
    realtime: bool,
}

impl FakeMediaSource {
    pub fn new() -> Self {
        Self { realtime: true }
    }

    pub fn fast() -> Self {
        Self { realtime: false }
    }
}

impl Default for FakeMediaSource {
    fn default() -> Self {
        Self::new()
    }
}

struct FakeSession {
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl MediaSession for FakeSession {
    fn request_keyframe(&self) {}

    fn stop(self: Box<Self>) {
        let mut this = self;
        this.stop.take();
        if let Some(thread) = this.thread.take() {
            thread.join().ok();
        }
    }
}

impl MediaSource for FakeMediaSource {
    fn start(
        &self,
        _target: CaptureTarget,
        _cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        let fixtures = fixtures()?;
        let (stop_tx, stop_rx) = bounded::<()>(0);
        let realtime = self.realtime;
        let t0 = mono_now();
        let thread = thread::Builder::new()
            .name("fake-media".into())
            .spawn(move || run(fixtures, realtime, t0, &out, &stop_rx))
            .map_err(|e| e.to_string())?;
        Ok(Box::new(FakeSession {
            stop: Some(stop_tx),
            thread: Some(thread),
        }))
    }
}

fn send(out: &Sender<MediaEvent>, stop: &Receiver<()>, event: MediaEvent) -> bool {
    select! {
        send(out, event) -> sent => sent.is_ok(),
        recv(stop) -> _ => false,
    }
}

fn ticks_to_hns(ticks: i64, timescale: i64) -> Hns {
    ticks * HNS_PER_SEC / timescale
}

fn run(
    fixtures: &Fixtures,
    realtime: bool,
    t0: Hns,
    out: &Sender<MediaEvent>,
    stop: &Receiver<()>,
) {
    let started = MediaEvent::Started {
        t0,
        video: fixtures.video.clone(),
        audio: Some(fixtures.audio.clone()),
        encoder: "fake".into(),
    };
    if !send(out, stop, started) {
        return;
    }
    let video_step = i64::from(fixtures.video.frame_ticks());
    let audio_step = i64::from(AAC_FRAME_TICKS);
    let (mut video_index, mut audio_index) = (0usize, 0usize);
    loop {
        let video_dts = video_index as i64 * video_step;
        let audio_dts = audio_index as i64 * audio_step;
        let video_at = ticks_to_hns(video_dts, i64::from(VideoFormat::TIMESCALE));
        let audio_at = ticks_to_hns(audio_dts, i64::from(SAMPLE_RATE));
        let (due, packet) = if video_at <= audio_at {
            let frame = &fixtures.video_frames[video_index % fixtures.video_frames.len()];
            video_index += 1;
            (
                video_at,
                Packet {
                    track: Track::Video,
                    dts: video_dts,
                    pts: video_dts,
                    duration: fixtures.video.frame_ticks(),
                    key: frame.key,
                    data: frame.data.clone(),
                },
            )
        } else {
            let data = &fixtures.audio_frames[audio_index % fixtures.audio_frames.len()];
            audio_index += 1;
            (
                audio_at,
                Packet {
                    track: Track::Audio,
                    dts: audio_dts,
                    pts: audio_dts,
                    duration: AAC_FRAME_TICKS,
                    key: true,
                    data: data.clone(),
                },
            )
        };
        if realtime && !wait_until(t0 + due, stop) {
            return;
        }
        if !send(out, stop, MediaEvent::Packet(packet)) {
            return;
        }
    }
}

fn wait_until(deadline: Hns, stop: &Receiver<()>) -> bool {
    loop {
        let remaining = deadline - mono_now();
        if remaining <= 0 {
            return true;
        }
        let wait = Duration::from_nanos(remaining as u64 * 100).min(IDLE_POLL);
        if stop.recv_timeout(wait).is_err_and(|e| e.is_disconnected()) {
            return false;
        }
    }
}

#[cfg(test)]
mod tests;
