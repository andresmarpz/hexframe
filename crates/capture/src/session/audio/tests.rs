use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use greyscreen_core::health::CaptureStats;
use greyscreen_core::media::MediaEvent;
use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now};

use super::{AudioPipe, spawn_worker};
use windows::Win32::Foundation::E_FAIL;

use crate::aac::{OpenEncoder, PcmEncoder, Result as AacResult};
use crate::audio::{OpenSource, PacketInfo, PcmBlock, SourceError};
use crate::clock::QpcMap;
use crate::fault::{Armed, Point};
use crate::session::emit::{Sink, StartInfo};
use crate::testing::client::{FakeClient, Step, failing_open as failing_source};
use crate::testing::encoder::{FakeEncoder, failing_open as failing_encoder};
use crate::testing::pcm::{click, sine};

const FRAMES: usize = 1024;
const PATIENCE: Duration = Duration::from_secs(5);

fn sink() -> (Sink, Receiver<MediaEvent>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut sink = Sink::new(
        tx,
        StartInfo {
            t0: 0,
            width: 16,
            height: 16,
            fps: 60,
            audio: None,
            encoder: "fake".into(),
        },
    );
    sink.start(Vec::new(), Vec::new());
    (sink, rx)
}

fn far_future_t0() -> Hns {
    mono_now() + 3600 * HNS_PER_SEC
}

fn start(source: OpenSource, encoder: OpenEncoder) -> Result<AudioPipe, String> {
    AudioPipe::start_with(source, encoder, QpcMap { offset: 0 }, 128_000)
}

fn sine_packets(count: usize) -> Vec<Step> {
    (0..count)
        .map(|index| Step::packet(sine(FRAMES, (index * FRAMES) as u64, 440.0)))
        .collect()
}

fn drain_until(pipe: &AudioPipe, sink: &mut Sink, want: u64) -> u64 {
    let deadline = Instant::now() + PATIENCE;
    let mut packets = 0;
    while packets < want && Instant::now() < deadline {
        packets += pipe.drain(sink);
        std::thread::sleep(Duration::from_millis(2));
    }
    packets
}

fn finish_within_patience(pipe: AudioPipe, mut sink: Sink) -> (u64, Sink) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let packets = pipe.finish(&mut sink);
        tx.send((packets, sink)).ok();
    });
    rx.recv_timeout(PATIENCE).expect("finish hung")
}

fn wait_until_stopped(stopped: &AtomicBool) {
    let deadline = Instant::now() + PATIENCE;
    while !stopped.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        stopped.load(Ordering::Acquire),
        "capture did not stop by itself"
    );
}

fn warnings(rx: &Receiver<MediaEvent>) -> Vec<String> {
    rx.try_iter()
        .filter_map(|event| match event {
            MediaEvent::Warning(text) => Some(text),
            _ => None,
        })
        .collect()
}

#[test]
fn queue_disconnects_when_the_worker_receiver_drops() {
    let (tx, worker) = spawn_worker(|_| {}).expect("worker spawns");
    worker.join().expect("worker ends");
    let block = PcmBlock {
        first_frame: 0,
        samples: vec![0; 4],
    };
    assert!(tx.send(block).is_err());
}

#[test]
fn pipe_encodes_every_scripted_packet_and_stops() {
    let client = FakeClient::new(sine_packets(5));
    let stopped = client.stopped();
    let encoder = FakeEncoder::new();
    let received = encoder.received();
    let pipe = start(client.open(), encoder.open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 5), 5);
    let (extra, _sink) = finish_within_patience(pipe, sink);
    assert_eq!(extra, 0);
    assert!(stopped.load(Ordering::Acquire));
    let blocks = received.lock().expect("received lock");
    assert_eq!(blocks.len(), 5);
    assert_eq!(blocks[3].first_frame, 3 * FRAMES as u64);
    assert!(blocks[0].samples.iter().any(|sample| *sample != 0));
}

#[test]
fn encoder_flush_packets_arrive_on_finish() {
    let client = FakeClient::new(sine_packets(2));
    let pipe = start(client.open(), FakeEncoder::new().flushing(3).open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 2), 2);
    let (flushed, _sink) = finish_within_patience(pipe, sink);
    assert_eq!(flushed, 3);
}

#[test]
fn encoder_failing_at_a_block_warns_and_the_pipe_still_stops() {
    let steps = (0..400)
        .map(|index| Step::packet(sine(64, index * 64, 440.0)))
        .collect::<Vec<_>>();
    let client = FakeClient::new(steps);
    let stopped = client.stopped();
    let pipe = start(client.open(), FakeEncoder::new().failing_at(2).open()).expect("pipe starts");
    let (mut sink, rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 2), 2);
    let (extra, _sink) = finish_within_patience(pipe, sink);
    assert_eq!(extra, 0);
    assert!(stopped.load(Ordering::Acquire));
    let texts = warnings(&rx);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].contains("audio encoding stopped"));
}

#[test]
fn slow_encoder_still_delivers_every_packet() {
    let client = FakeClient::new(sine_packets(20));
    let encoder = FakeEncoder::new().sleeping(Duration::from_millis(3));
    let pipe = start(client.open(), encoder.open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 20), 20);
    finish_within_patience(pipe, sink);
}

#[test]
fn device_invalidated_ends_capture_and_keeps_earlier_packets() {
    let mut steps = sine_packets(3);
    steps.push(Step::ReadFails(SourceError::DeviceInvalidated));
    let client = FakeClient::new(steps);
    let stopped = client.stopped();
    let pipe = start(client.open(), FakeEncoder::new().open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 3), 3);
    wait_until_stopped(&stopped);
    let (extra, _sink) = finish_within_patience(pipe, sink);
    assert_eq!(extra, 0);
}

#[test]
fn read_failure_ends_capture_without_hanging() {
    let mut steps = sine_packets(2);
    steps.push(Step::ReadFails(SourceError::Failed("scripted".into())));
    steps.extend(sine_packets(2));
    let client = FakeClient::new(steps);
    let stopped = client.stopped();
    let encoder = FakeEncoder::new();
    let received = encoder.received();
    let pipe = start(client.open(), encoder.open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 2), 2);
    wait_until_stopped(&stopped);
    let (extra, _sink) = finish_within_patience(pipe, sink);
    assert_eq!(extra, 0);
    assert_eq!(received.lock().expect("received lock").len(), 2);
}

#[test]
fn discontinuity_flags_are_counted_and_silent_packets_encode_as_silence() {
    let discontinuous = PacketInfo {
        discontinuity: true,
        ..PacketInfo::default()
    };
    let silent = PacketInfo {
        silent: true,
        ..PacketInfo::default()
    };
    let steps = vec![
        Step::flagged(click(FRAMES, 10), discontinuous),
        Step::flagged(sine(FRAMES, 0, 440.0), silent),
        Step::Idle,
        Step::packet(click(FRAMES, 0)),
    ];
    let encoder = FakeEncoder::new();
    let received = encoder.received();
    let pipe = start(FakeClient::new(steps).open(), encoder.open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 3), 3);
    let mut stats = CaptureStats::default();
    pipe.fill_stats(&mut stats);
    assert_eq!(stats.audio_discontinuities, 1);
    finish_within_patience(pipe, sink);
    let blocks = received.lock().expect("received lock");
    assert_eq!(blocks[0].samples[10 * 2], i16::MAX);
    assert!(blocks[1].samples.iter().all(|sample| *sample == 0));
    assert_eq!(blocks[2].samples[0], i16::MAX);
}

#[test]
fn a_source_that_fails_to_open_fails_the_pipe_start() {
    let Err(error) = start(failing_source(), FakeEncoder::new().open()) else {
        panic!("start should fail");
    };
    assert!(error.contains("audio capture failed"));
}

#[test]
fn an_encoder_that_fails_to_open_fails_the_pipe_start() {
    let client = FakeClient::new(Vec::new());
    let Err(error) = start(client.open(), failing_encoder()) else {
        panic!("start should fail");
    };
    assert!(error.contains("AAC encoder failed"));
}

struct Tracked {
    inner: Option<Box<dyn PcmEncoder>>,
    alive: Arc<AtomicBool>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}

impl PcmEncoder for Tracked {
    fn encode(&mut self, block: &PcmBlock, out: &mut Vec<Vec<u8>>) -> AacResult<()> {
        self.inner
            .as_mut()
            .expect("inner encoder")
            .encode(block, out)
    }

    fn finish(mut self: Box<Self>, out: &mut Vec<Vec<u8>>) -> AacResult<()> {
        self.inner.take().expect("inner encoder").finish(out)
    }

    fn asc(&self) -> Vec<u8> {
        self.inner.as_ref().expect("inner encoder").asc()
    }
}

fn tracked(encoder: FakeEncoder) -> (OpenEncoder, Arc<AtomicBool>) {
    let alive = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&alive);
    let open = encoder.open();
    let wrapped: OpenEncoder = Box::new(move || {
        let inner = open()?;
        Ok(Box::new(Tracked {
            inner: Some(inner),
            alive: flag,
        }))
    });
    (wrapped, alive)
}

fn wait_until_gone(alive: &AtomicBool) {
    let deadline = Instant::now() + PATIENCE;
    while alive.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        !alive.load(Ordering::Acquire),
        "the AAC thread did not exit"
    );
}

fn blocks(count: usize) -> Vec<Step> {
    (0..count)
        .map(|index| Step::packet(sine(64, index as u64 * 64, 440.0)))
        .collect()
}

#[test]
fn a_start_that_fails_after_the_aac_thread_started_joins_it_and_a_second_start_works() {
    let (encoder, alive) = tracked(FakeEncoder::new());
    let Err(error) = start(failing_source(), encoder) else {
        panic!("start should fail");
    };
    assert!(error.contains("audio capture failed"));
    assert!(
        !alive.load(Ordering::Acquire),
        "the AAC thread was not joined"
    );
    let pipe = start(
        FakeClient::new(sine_packets(1)).open(),
        FakeEncoder::new().open(),
    )
    .expect("second start works");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 1), 1);
    finish_within_patience(pipe, sink);
}

#[test]
fn stopping_before_the_session_starts_returns_at_once_and_sends_nothing() {
    let client = FakeClient::new(sine_packets(10));
    let stopped = client.stopped();
    let pipe = start(client.open(), FakeEncoder::new().open()).expect("pipe starts");
    let (sink, rx) = sink();
    let begun = Instant::now();
    let (packets, _sink) = finish_within_patience(pipe, sink);
    assert!(begun.elapsed() < Duration::from_secs(1));
    assert_eq!(packets, 0);
    assert!(stopped.load(Ordering::Acquire));
    assert!(
        rx.try_iter()
            .all(|event| !matches!(event, MediaEvent::Packet(_)))
    );
}

#[test]
fn dropping_the_pipe_without_finish_joins_every_thread() {
    let client = FakeClient::new(sine_packets(10));
    let stopped = client.stopped();
    let (encoder, alive) = tracked(FakeEncoder::new());
    let pipe = start(client.open(), encoder).expect("pipe starts");
    pipe.begin(far_future_t0());
    drop(pipe);
    assert!(stopped.load(Ordering::Acquire));
    assert!(
        !alive.load(Ordering::Acquire),
        "the AAC thread was not joined"
    );
}

#[test]
fn a_source_that_cannot_open_gives_a_cause_the_session_can_show() {
    let Err(error) = start(failing_source(), FakeEncoder::new().open()) else {
        panic!("start should fail");
    };
    assert!(error.starts_with("audio capture failed: "));
    assert!(error.len() > "audio capture failed: ".len());
}

#[test]
fn two_hundred_fast_start_and_stop_cycles_do_not_hang() {
    let begun = Instant::now();
    for cycle in 0..200 {
        let pipe = start(
            FakeClient::new(sine_packets(2)).open(),
            FakeEncoder::new().open(),
        )
        .expect("pipe starts");
        if cycle % 2 == 0 {
            pipe.begin(far_future_t0());
        }
        let (sink, _rx) = sink();
        finish_within_patience(pipe, sink);
    }
    assert!(begun.elapsed() < Duration::from_secs(60));
}

#[test]
fn an_encoder_failing_on_the_first_block_ends_the_audio_thread_within_a_second_of_stop() {
    let client = FakeClient::new(blocks(300));
    let stopped = client.stopped();
    let (encoder, alive) = tracked(FakeEncoder::new().failing_at(0));
    let pipe = start(client.open(), encoder).expect("pipe starts");
    let (sink, rx) = sink();
    pipe.begin(far_future_t0());
    wait_until_gone(&alive);
    let gone = Instant::now();
    wait_until_stopped(&stopped);
    assert!(gone.elapsed() < Duration::from_secs(1));
    let begun = Instant::now();
    let (_, _sink) = finish_within_patience(pipe, sink);
    assert!(begun.elapsed() < Duration::from_secs(2));
    assert_eq!(warnings(&rx).len(), 1);
}

#[test]
fn an_encoder_failing_on_block_fifty_keeps_the_first_packets_warns_once_and_stops_fast() {
    let client = FakeClient::new(blocks(300));
    let pipe = start(client.open(), FakeEncoder::new().failing_at(50).open()).expect("pipe starts");
    let (mut sink, rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 50), 50);
    let begun = Instant::now();
    let (extra, _sink) = finish_within_patience(pipe, sink);
    assert!(begun.elapsed() < Duration::from_secs(2));
    assert_eq!(extra, 0);
    let texts = warnings(&rx);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].contains("audio encoding stopped"));
}

#[test]
fn a_stalled_encoder_fills_the_queue_to_its_bound_and_audio_resumes_in_place_by_timestamp() {
    const PACKET_FRAMES: u64 = 64;
    const JUMP_FRAMES: u64 = 24_000;
    const JUMP_AT: u64 = 480;
    let hns_per_sec = HNS_PER_SEC as u64;
    let t0 = mono_now();
    let timestamp = |frames: u64| t0 + (frames * hns_per_sec / 48_000) as Hns;
    let steps: Vec<Step> = (0..700u64)
        .map(|index| {
            let jump = if index >= JUMP_AT { JUMP_FRAMES } else { 0 };
            Step::flagged(
                sine(64, index * PACKET_FRAMES, 440.0),
                PacketInfo {
                    timestamp: timestamp(index * PACKET_FRAMES + jump) as u64,
                    ..PacketInfo::default()
                },
            )
        })
        .collect();
    let client = FakeClient::new(steps);
    let encoder = FakeEncoder::new().stalling_at(0, Duration::from_secs(2));
    let received = encoder.received();
    let pipe = start(client.open(), encoder.open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(t0);
    let mut deepest = 0;
    let until = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < until {
        let mut stats = CaptureStats::default();
        pipe.fill_stats(&mut stats);
        deepest = deepest.max(stats.pcm_queue);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(deepest as usize, super::PCM_QUEUE);
    assert_eq!(drain_until(&pipe, &mut sink, 700), 700);
    let mut stats = CaptureStats::default();
    pipe.fill_stats(&mut stats);
    assert_eq!(stats.audio_gaps, 1);
    finish_within_patience(pipe, sink);
    let got = received.lock().expect("received lock");
    assert_eq!(got.len(), 701);
    let jump_at = JUMP_AT as usize;
    let jump_start = JUMP_AT * PACKET_FRAMES;
    assert_eq!(got[jump_at].first_frame, jump_start);
    assert_eq!(got[jump_at].samples.len(), JUMP_FRAMES as usize * 2);
    assert!(got[jump_at].samples.iter().all(|sample| *sample == 0));
    let resumed_at = (timestamp(jump_start + JUMP_FRAMES) - t0) as u64 * 48_000 / hns_per_sec;
    assert_eq!(got[jump_at + 1].first_frame, resumed_at);
    assert!(
        got.windows(2)
            .all(|pair| pair[0].first_frame < pair[1].first_frame)
    );
}

#[test]
fn device_invalidated_sends_one_warning_to_the_session() {
    let mut steps = sine_packets(3);
    steps.push(Step::ReadFails(SourceError::DeviceInvalidated));
    let pipe =
        start(FakeClient::new(steps).open(), FakeEncoder::new().open()).expect("pipe starts");
    let (mut sink, rx) = sink();
    pipe.begin(far_future_t0());
    drain_until(&pipe, &mut sink, 3);
    let deadline = Instant::now() + PATIENCE;
    let mut texts = Vec::new();
    while texts.is_empty() && Instant::now() < deadline {
        pipe.drain(&mut sink);
        texts = warnings(&rx);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(texts.len(), 1);
    finish_within_patience(pipe, sink);
}

#[test]
fn device_loss_while_the_event_never_fires_sends_one_warning() {
    let mut steps = sine_packets(3);
    steps.push(Step::SilentLoss(SourceError::DeviceInvalidated));
    let pipe =
        start(FakeClient::new(steps).open(), FakeEncoder::new().open()).expect("pipe starts");
    let (mut sink, rx) = sink();
    pipe.begin(far_future_t0());
    drain_until(&pipe, &mut sink, 3);
    let deadline = Instant::now() + PATIENCE;
    let mut texts = Vec::new();
    while texts.is_empty() && Instant::now() < deadline {
        pipe.drain(&mut sink);
        texts = warnings(&rx);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(texts.len(), 1);
    finish_within_patience(pipe, sink);
}

#[test]
fn a_failed_mmcss_registration_still_captures_audio() {
    let _armed = Armed::new().fail(Point::Mmcss, E_FAIL);
    let pipe = start(
        FakeClient::new(sine_packets(5)).open(),
        FakeEncoder::new().open(),
    )
    .expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(far_future_t0());
    assert_eq!(drain_until(&pipe, &mut sink, 5), 5);
    finish_within_patience(pipe, sink);
}

fn timestamp_of(frame: u64, t0: Hns) -> u64 {
    (t0 + (frame * HNS_PER_SEC as u64 / 48_000) as Hns) as u64
}

fn stamped(index: u64, t0: Hns, mut info: PacketInfo, pcm: Vec<f32>) -> Step {
    if !info.timestamp_error {
        info.timestamp = timestamp_of(index * FRAMES as u64, t0);
    }
    Step::flagged(pcm, info)
}

fn cold_sink(t0: Hns) -> (Sink, Receiver<MediaEvent>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let sink = Sink::new(
        tx,
        StartInfo {
            t0,
            width: 16,
            height: 16,
            fps: 60,
            audio: None,
            encoder: "fake".into(),
        },
    );
    (sink, rx)
}

#[test]
fn a_timestamp_jump_mid_run_is_counted_measured_and_filled_with_silence_in_one_second_chunks() {
    const JUMP_SECS: u64 = 30;
    let t0 = mono_now();
    let jump_frames = JUMP_SECS * 48_000;
    let info = |frame: u64| PacketInfo {
        timestamp: timestamp_of(frame, t0),
        ..PacketInfo::default()
    };
    let steps = vec![
        Step::flagged(sine(FRAMES, 0, 440.0), info(0)),
        Step::flagged(
            sine(FRAMES, FRAMES as u64, 440.0),
            info(FRAMES as u64 + jump_frames),
        ),
    ];
    let encoder = FakeEncoder::new();
    let received = encoder.received();
    let pipe = start(FakeClient::new(steps).open(), encoder.open()).expect("pipe starts");
    let (mut sink, rx) = sink();
    pipe.begin(t0);
    assert_eq!(drain_until(&pipe, &mut sink, 2 + JUMP_SECS), 2 + JUMP_SECS);
    let mut stats = CaptureStats::default();
    pipe.fill_stats(&mut stats);
    assert_eq!(stats.audio_timestamp_gaps_over_5s, 1);
    assert!(
        stats
            .audio_largest_timestamp_gap_ms
            .abs_diff(JUMP_SECS * 1000)
            <= 100
    );
    assert_eq!(stats.audio_gaps, 1);
    assert!(stats.audio_gap_total_ms.abs_diff(JUMP_SECS * 1000) <= 100);
    finish_within_patience(pipe, sink);
    let blocks = received.lock().expect("received lock");
    assert!(blocks.iter().all(|block| block.samples.len() <= 48_000 * 2));
    let full_seconds = blocks
        .iter()
        .filter(|block| block.samples.len() == 48_000 * 2)
        .count();
    assert_eq!(full_seconds as u64, JUMP_SECS);
    assert!(warnings(&rx).is_empty());
}

fn assert_idle_tail(blocks: &[PcmBlock], scripted_frames: u64) {
    for block in blocks {
        assert!(block.first_frame >= scripted_frames);
        assert!(block.samples.iter().all(|sample| *sample == 0));
    }
    assert!(
        blocks
            .windows(2)
            .all(|pair| pair[0].first_frame < pair[1].first_frame)
    );
}

fn is_sine_block(block: &PcmBlock, first_frame: u64) -> bool {
    let expected: Vec<i16> = sine(FRAMES, first_frame, 440.0)
        .into_iter()
        .map(super::f32_to_i16)
        .collect();
    block.first_frame == first_frame && block.samples == expected
}

#[test]
fn audio_before_the_first_video_frame_is_held_until_started_and_nothing_is_lost() {
    const ON_TIME: u64 = 4;
    let t0 = mono_now() + HNS_PER_SEC / 10;
    let early = PacketInfo {
        timestamp: (t0 - HNS_PER_SEC / 10) as u64,
        ..PacketInfo::default()
    };
    let mut steps = vec![Step::flagged(sine(FRAMES, 777_777, 880.0), early)];
    steps.extend((0..ON_TIME).map(|index| {
        let first = index * FRAMES as u64;
        stamped(index, t0, PacketInfo::default(), sine(FRAMES, first, 440.0))
    }));
    let encoder = FakeEncoder::new();
    let received = encoder.received();
    let pipe = start(FakeClient::new(steps).open(), encoder.open()).expect("pipe starts");
    let (mut sink, rx) = cold_sink(t0);
    pipe.begin(t0);
    assert!(drain_until(&pipe, &mut sink, ON_TIME) >= ON_TIME);
    assert!(rx.try_recv().is_err());
    sink.start(Vec::new(), Vec::new());
    finish_within_patience(pipe, sink);
    let events: Vec<MediaEvent> = rx.try_iter().collect();
    assert!(matches!(events.first(), Some(MediaEvent::Started { .. })));
    let audio_packets = events
        .iter()
        .filter(|event| matches!(event, MediaEvent::Packet(_)))
        .count();
    assert!(audio_packets as u64 >= ON_TIME);
    let blocks = received.lock().expect("received lock");
    assert!(blocks.len() as u64 >= ON_TIME);
    for (index, block) in blocks.iter().take(ON_TIME as usize).enumerate() {
        assert!(
            is_sine_block(block, index as u64 * FRAMES as u64),
            "block {index}"
        );
    }
    assert_idle_tail(&blocks[ON_TIME as usize..], ON_TIME * FRAMES as u64);
}

#[test]
fn error_silent_and_discontinuity_flags_follow_the_buffer_flag_policy_and_never_reset_the_stream() {
    const SCRIPTED: usize = 5;
    let t0 = mono_now() + HNS_PER_SEC / 10;
    let plain = PacketInfo::default();
    let errored = PacketInfo {
        timestamp_error: true,
        timestamp: (t0 + 3600 * HNS_PER_SEC) as u64,
        ..plain
    };
    let silent = PacketInfo {
        silent: true,
        ..plain
    };
    let discontinuous = PacketInfo {
        discontinuity: true,
        ..plain
    };
    let infos = [plain, errored, silent, discontinuous, plain];
    let steps: Vec<Step> = infos
        .iter()
        .enumerate()
        .map(|(index, info)| {
            let first = index as u64 * FRAMES as u64;
            stamped(index as u64, t0, *info, sine(FRAMES, first, 440.0))
        })
        .collect();
    let encoder = FakeEncoder::new();
    let received = encoder.received();
    let pipe = start(FakeClient::new(steps).open(), encoder.open()).expect("pipe starts");
    let (mut sink, _rx) = sink();
    pipe.begin(t0);
    assert!(drain_until(&pipe, &mut sink, SCRIPTED as u64) >= SCRIPTED as u64);
    let mut stats = CaptureStats::default();
    pipe.fill_stats(&mut stats);
    assert_eq!(stats.audio_discontinuities, 1);
    assert_eq!(stats.audio_gaps, 0);
    assert_eq!(stats.audio_timestamp_gaps_over_5s, 0);
    finish_within_patience(pipe, sink);
    let blocks = received.lock().expect("received lock");
    let scripted_frames = (SCRIPTED * FRAMES) as u64;
    let split = blocks.partition_point(|block| block.first_frame < scripted_frames);
    let (scripted, tail) = blocks.split_at(split);
    assert_eq!(scripted.len(), SCRIPTED);
    for (index, block) in scripted.iter().enumerate() {
        assert_eq!(block.first_frame, (index * FRAMES) as u64);
        assert_eq!(block.samples.len(), FRAMES * 2);
        let zeroed = block.samples.iter().all(|sample| *sample == 0);
        assert_eq!(zeroed, index == 2, "block {index}");
    }
    assert_idle_tail(tail, scripted_frames);
}

#[test]
fn ten_seconds_through_the_real_aac_encoder_and_the_muxer_keep_the_encoders_rate() {
    use greyscreen_core::media::{Packet, Track, VideoFormat};
    use greyscreen_mp4::fragmented::FragmentedWriter;
    use greyscreen_mp4::inspect::{inspect, sample_secs};

    const PACKETS: usize = 10 * 48_000 / FRAMES + 1;
    const SAMPLING_INDEX_48K: u8 = 3;
    let encoder = crate::aac::open_aac(48_000, 2, 128_000);
    let source = FakeClient::new(sine_packets(PACKETS)).open();
    let pipe = start(source, encoder).expect("pipe starts");
    let format = pipe.format.clone();
    let (mut sink, rx) = sink();
    pipe.begin(mono_now());
    let wanted = (9 * 48_000 / FRAMES) as u64;
    assert!(drain_until(&pipe, &mut sink, wanted) >= wanted);
    finish_within_patience(pipe, sink);
    let audio: Vec<Packet> = rx
        .try_iter()
        .filter_map(|event| match event {
            MediaEvent::Packet(packet) => Some(packet),
            _ => None,
        })
        .collect();
    assert_eq!(format.sample_rate, 48_000);
    let asc = &format.asc;
    assert_eq!(((asc[0] & 7) << 1) | (asc[1] >> 7), SAMPLING_INDEX_48K);

    let path = std::env::temp_dir().join(format!("greyscreen-aac-rate-{}.mp4", std::process::id()));
    let video = VideoFormat {
        width: 640,
        height: 360,
        fps: 60,
        sps: vec![0x67, 0x42, 0x00, 0x1f],
        pps: vec![0x68, 0xce, 0x3c, 0x80],
    };
    let mut writer = FragmentedWriter::create(&path, &video, Some(&format)).expect("writer");
    for (index, key) in [(0i64, true), (1, false)] {
        writer
            .push(&Packet {
                track: Track::Video,
                dts: index * 1500,
                pts: index * 1500,
                duration: 1500,
                key,
                data: Arc::from(vec![7u8; 100]),
            })
            .expect("video packet");
    }
    for packet in &audio {
        writer.push(packet).expect("audio packet");
    }
    writer.finish().expect("finish");
    let inspected = inspect(&path).expect("inspect");
    std::fs::remove_file(&path).ok();
    let track = inspected.audio.expect("audio track");
    assert_eq!(track.sample_rate, format.sample_rate);
    assert_eq!(&track.asc, asc);
    let secs = sample_secs(&inspected.tables.audio, track.sample_rate);
    assert!((9.0..=10.1).contains(&secs), "{secs}");
}
