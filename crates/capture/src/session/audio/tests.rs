use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use greyscreen_core::health::CaptureStats;
use greyscreen_core::media::MediaEvent;
use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now};

use super::{AudioPipe, spawn_worker};
use crate::aac::OpenEncoder;
use crate::audio::{OpenSource, PacketInfo, PcmBlock, SourceError};
use crate::clock::QpcMap;
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
