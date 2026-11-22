use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use greyscreen_core::health::CaptureStats;

use super::{
    AudioCounters, AudioTimeline, CHANNELS, CaptureClient, PacketInfo, PcmBlock, RATE, SourceError,
    drain, send_block,
};
use crate::clock::{QpcMap, mono_now};

fn block() -> PcmBlock {
    PcmBlock {
        first_frame: 0,
        samples: vec![0; 4],
    }
}

#[test]
fn send_on_a_full_queue_returns_soon_after_stop() {
    let (tx, _rx) = crossbeam_channel::bounded(1);
    let counters = AudioCounters::default();
    let stop = AtomicBool::new(false);
    assert!(send_block(&tx, block(), &stop, &counters));
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(50));
            stop.store(true, Ordering::Release);
        });
        assert!(!send_block(&tx, block(), &stop, &counters));
    });
    let after_stop = AtomicBool::new(true);
    let started = Instant::now();
    assert!(!send_block(&tx, block(), &after_stop, &counters));
    assert!(started.elapsed() < Duration::from_millis(100));
}

#[test]
fn send_returns_at_once_when_the_receiver_is_gone() {
    let (tx, rx) = crossbeam_channel::bounded(1);
    let counters = AudioCounters::default();
    let stop = AtomicBool::new(false);
    assert!(send_block(&tx, block(), &stop, &counters));
    drop(rx);
    let started = Instant::now();
    assert!(!send_block(&tx, block(), &stop, &counters));
    assert!(started.elapsed() < Duration::from_millis(100));
}

#[test]
fn queue_depth_counts_queued_blocks() {
    let (tx, _rx) = crossbeam_channel::bounded(8);
    let counters = AudioCounters::default();
    let stop = AtomicBool::new(false);
    for _ in 0..3 {
        assert!(send_block(&tx, block(), &stop, &counters));
    }
    let mut stats = CaptureStats::default();
    counters.fill(&mut stats);
    assert_eq!(stats.pcm_queue, 3);
}

struct FakeClient {
    packets: Vec<(u32, PacketInfo)>,
}

impl CaptureClient for FakeClient {
    fn wait(&mut self, _timeout_ms: u32) -> Result<bool, SourceError> {
        Ok(true)
    }

    fn next_packet_size(&mut self) -> Result<Option<u32>, SourceError> {
        Ok(self.packets.first().map(|(frames, _)| *frames))
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<(u32, PacketInfo), SourceError> {
        let (frames, info) = self.packets.remove(0);
        buffer.fill(0);
        Ok((frames, info))
    }

    fn stop(&mut self) {}
}

#[test]
fn one_hour_forward_jump_is_sent_in_blocks_of_at_most_one_second() {
    const HOUR: i64 = 3600 * 10_000_000;
    let t0 = mono_now();
    let packet = |timestamp: i64| {
        (
            480,
            PacketInfo {
                timestamp: timestamp as u64,
                ..PacketInfo::default()
            },
        )
    };
    let mut client = FakeClient {
        packets: vec![packet(t0), packet(t0 + 100_000 + HOUR)],
    };
    let mut timeline = AudioTimeline::new(t0, RATE).expect("rate");
    let (tx, rx) = crossbeam_channel::unbounded();
    let counters = AudioCounters::default();
    let stop = AtomicBool::new(false);
    let mut buffer = Vec::new();
    let qpc = QpcMap { offset: 0 };
    assert!(
        drain(
            &mut client,
            qpc,
            &mut timeline,
            &mut buffer,
            &tx,
            &counters,
            &stop
        )
        .expect("drain")
    );
    drop(tx);
    let blocks: Vec<PcmBlock> = rx.iter().collect();
    assert!(
        blocks
            .iter()
            .all(|b| b.samples.len() <= RATE as usize * CHANNELS)
    );
    let mut next = 0;
    for block in &blocks {
        assert_eq!(block.first_frame, next);
        next += (block.samples.len() / CHANNELS) as u64;
    }
    assert_eq!(next, timeline.frames_out());
    assert!(next >= 3600 * u64::from(RATE));
}
