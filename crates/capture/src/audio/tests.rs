use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use greyscreen_core::health::CaptureStats;

use super::{AudioCounters, PcmBlock, send_block};

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
