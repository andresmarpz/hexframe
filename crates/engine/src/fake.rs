use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, bounded, select};
use greyscreen_core::health::{CaptureSetup, CaptureStats};
use greyscreen_core::media::{
    AudioFormat, CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource, Packet,
    Track, VideoFormat,
};
use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now, secs_to_hns};
use greyscreen_mp4::aac::{asc_lc, split_adts};
use greyscreen_mp4::avc::{
    NAL_AUD, NAL_PPS, NAL_SEI, NAL_SPS, annexb_to_avcc, nal_type, parse_sps_dims, split_annexb,
};

mod faults;

use faults::LiveGuard;
pub use faults::{FailAt, FakeProbe, Faults};

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

const STOP_BOUND: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct FakeMediaSource {
    realtime: bool,
    speed: u32,
    start_offset_secs: u32,
    faults: Faults,
    stop_bound: Duration,
    probe: Arc<FakeProbe>,
}

impl FakeMediaSource {
    pub fn new() -> Self {
        Self {
            realtime: true,
            speed: 1,
            start_offset_secs: 0,
            faults: Faults::default(),
            stop_bound: STOP_BOUND,
            probe: Arc::default(),
        }
    }

    pub fn fast() -> Self {
        Self {
            realtime: false,
            ..Self::new()
        }
    }

    pub fn with_speed(self, speed: u32) -> Self {
        Self {
            speed: speed.max(1),
            ..self
        }
    }

    pub fn with_start_offset_secs(self, secs: u32) -> Self {
        Self {
            start_offset_secs: secs,
            ..self
        }
    }

    pub fn failing_after(self, delay: Duration, reason: &str) -> Self {
        self.with_faults(|faults| faults.fail = Some((FailAt::After(delay), reason.to_owned())))
    }

    pub fn failing_after_packets(self, packets: u32, reason: &str) -> Self {
        self.with_faults(|faults| {
            faults.fail = Some((FailAt::AfterPackets(packets), reason.to_owned()));
        })
    }

    pub fn failing_to_start(self, reason: &str) -> Self {
        self.with_faults(|faults| faults.start_error = Some(reason.to_owned()))
    }

    pub fn blocking_start(self, time: Duration) -> Self {
        self.with_faults(|faults| faults.start_block = time)
    }

    pub fn blocking_stop(self, time: Duration) -> Self {
        self.with_faults(|faults| faults.stop_block = time)
    }

    pub fn with_stop_bound(self, bound: Duration) -> Self {
        Self {
            stop_bound: bound,
            ..self
        }
    }

    pub fn with_audio_delay_secs(self, secs: u32) -> Self {
        self.with_faults(|faults| faults.audio_delay_secs = secs)
    }

    pub fn stopping_audio_after(self, packets: u32, warning: &str) -> Self {
        self.with_faults(|faults| faults.audio_stop = Some((packets, warning.to_owned())))
    }

    pub fn with_final_stats(self) -> Self {
        self.with_faults(|faults| faults.final_stats = true)
    }

    pub fn probe(&self) -> Arc<FakeProbe> {
        self.probe.clone()
    }

    fn with_faults(mut self, change: impl FnOnce(&mut Faults)) -> Self {
        change(&mut self.faults);
        self
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
    stop_block: Duration,
    _live: LiveGuard,
}

impl MediaSession for FakeSession {
    fn request_keyframe(&self) {}

    fn stop(self: Box<Self>) {
        let mut this = self;
        this.stop.take();
        thread::sleep(this.stop_block);
        if let Some(thread) = this.thread.take() {
            thread.join().ok();
        }
    }
}

impl MediaSource for FakeMediaSource {
    fn start(
        &self,
        _target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        let fixtures = fixtures()?;
        if let Some(reason) = &self.faults.start_error {
            return Err(reason.clone());
        }
        thread::sleep(self.faults.start_block);
        let (stop_tx, stop_rx) = bounded::<()>(0);
        let plan = Plan {
            realtime: self.realtime,
            speed: i64::from(self.speed),
            start_offset_secs: i64::from(self.start_offset_secs),
            faults: self.faults.clone(),
            probe: self.probe.clone(),
        };
        let stop_block = plan.faults.stop_block.min(self.stop_bound);
        let live = self.probe.session_started();
        let t0 = mono_now();
        let thread = thread::Builder::new()
            .name("fake-media".into())
            .spawn(move || run(fixtures, &plan, t0, cfg.diagnostics, &out, &stop_rx))
            .map_err(|e| e.to_string())?;
        Ok(Box::new(FakeSession {
            stop: Some(stop_tx),
            thread: Some(thread),
            stop_block,
            _live: live,
        }))
    }
}

struct Link<'a> {
    out: &'a Sender<MediaEvent>,
    stop: &'a Receiver<()>,
    probe: &'a FakeProbe,
}

impl Link<'_> {
    fn send(&self, event: MediaEvent) -> bool {
        let began = Instant::now();
        let is_packet = matches!(event, MediaEvent::Packet(_));
        let sent = select! {
            send(self.out, event) -> sent => sent.is_ok(),
            recv(self.stop) -> _ => false,
        };
        self.probe.record_send_wait(began.elapsed());
        if sent && is_packet {
            self.probe.record_packet();
        }
        sent
    }
}

fn ticks_to_hns(ticks: i64, timescale: i64) -> Hns {
    ticks * HNS_PER_SEC / timescale
}

struct Plan {
    realtime: bool,
    speed: i64,
    start_offset_secs: i64,
    faults: Faults,
    probe: Arc<FakeProbe>,
}

fn failure_due(at: FailAt, packets: u32, t0: Hns) -> bool {
    match at {
        FailAt::After(delay) => mono_now() - t0 >= secs_to_hns(delay.as_secs_f64()),
        FailAt::AfterPackets(limit) => packets >= limit,
    }
}

fn run(
    fixtures: &Fixtures,
    plan: &Plan,
    t0: Hns,
    diagnostics: bool,
    out: &Sender<MediaEvent>,
    stop: &Receiver<()>,
) {
    let link = Link {
        out,
        stop,
        probe: &plan.probe,
    };
    let mut stats = CaptureStats::default();
    produce(fixtures, plan, t0, diagnostics, &link, &mut stats);
    if plan.faults.final_stats {
        out.try_send(MediaEvent::Stats(Box::new(stats))).ok();
    }
}

fn produce(
    fixtures: &Fixtures,
    plan: &Plan,
    t0: Hns,
    diagnostics: bool,
    link: &Link,
    stats: &mut CaptureStats,
) {
    let started = MediaEvent::Started {
        t0,
        video: fixtures.video.clone(),
        audio: Some(fixtures.audio.clone()),
        encoder: "fake".into(),
    };
    if !link.send(started) {
        return;
    }
    if diagnostics {
        let setup = CaptureSetup {
            adapter: "Fake GPU".into(),
            fps: fixtures.video.fps,
            encode_width: fixtures.video.width,
            encode_height: fixtures.video.height,
            encoder: "fake".into(),
            audio: true,
            ..CaptureSetup::default()
        };
        if !link.send(MediaEvent::Setup(Box::new(setup))) {
            return;
        }
    }
    let mut next_stats = HNS_PER_SEC;
    let video_step = i64::from(fixtures.video.frame_ticks());
    let audio_step = i64::from(AAC_FRAME_TICKS);
    let video_start = plan.start_offset_secs * i64::from(VideoFormat::TIMESCALE);
    let audio_offset_secs = plan.start_offset_secs + i64::from(plan.faults.audio_delay_secs);
    let audio_start = audio_offset_secs * i64::from(SAMPLE_RATE);
    let (mut video_index, mut audio_index) = (0usize, 0usize);
    let mut sent_packets = 0u32;
    let mut failure = plan.faults.fail.clone();
    let mut audio_warned = false;
    loop {
        let audio_done = plan
            .faults
            .audio_stop
            .as_ref()
            .is_some_and(|(limit, _)| audio_index >= *limit as usize);
        if audio_done
            && !audio_warned
            && let Some((_, warning)) = &plan.faults.audio_stop
        {
            audio_warned = true;
            if !link.send(MediaEvent::Warning(warning.clone())) {
                return;
            }
        }
        let video_dts = video_start + video_index as i64 * video_step;
        let audio_dts = audio_start + audio_index as i64 * audio_step;
        let video_at = ticks_to_hns(video_dts, i64::from(VideoFormat::TIMESCALE));
        let audio_at = ticks_to_hns(audio_dts, i64::from(SAMPLE_RATE));
        let (due, packet) = if video_at <= audio_at || audio_done {
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
        if plan.realtime && !wait_until(t0 + due / plan.speed, link.stop) {
            return;
        }
        let is_video = packet.track == Track::Video;
        if !link.send(MediaEvent::Packet(packet)) {
            return;
        }
        sent_packets += 1;
        if is_video {
            stats.captured += 1;
            stats.ticks += 1;
            stats.encode.record(2_000);
        }
        if let Some((at, reason)) = &failure
            && failure_due(*at, sent_packets, t0)
        {
            if !link.send(MediaEvent::Failed(reason.clone())) {
                return;
            }
            failure = None;
        }
        if due >= next_stats {
            next_stats += HNS_PER_SEC;
            stats.at = t0 + due;
            if !link.send(MediaEvent::Stats(Box::new(stats.clone()))) {
                return;
            }
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
