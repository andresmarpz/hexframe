use super::super::{Chunk, Clock, NUDGE, Queue, REREAD, run};
use super::*;
use crate::video::windows::machine::Anchor;
use std::sync::Arc;

const RATE: u32 = 48_000;

/// Three seconds of mono sound whose every sample is its own media time.
fn timed_queue() -> Arc<Queue> {
    let queue = Arc::new(Queue::default());
    let mut feed = queue.lock();
    feed.gain = 1.0;
    let chunk = RATE as usize / 100;
    for c in 0..300 {
        let samples: Vec<f32> = (0..chunk).map(|i| ((c * chunk + i) as f64 / f64::from(RATE)) as f32).collect();
        feed.samples += samples.len();
        feed.chunks.push_back(Chunk { pts: (c * chunk) as f64 / f64::from(RATE), samples });
    }
    drop(feed);
    queue
}

fn play(queue: &Arc<Queue>, clock: &Arc<Clock>) -> std::thread::JoinHandle<()> {
    let (queue, clock) = (queue.clone(), clock.clone());
    std::thread::spawn(move || run(&Fake::new(Sound { channels: 1, rate: RATE }), &queue, &clock, Sound { channels: 1, rate: RATE }))
}

/// The heard frames after `from`: when each was heard and the media time it carries.
fn heard_after(heard: &[Heard], from: Instant) -> Vec<(Instant, f64)> {
    heard
        .iter()
        .flat_map(|h| h.samples.iter().enumerate().map(move |(i, x)| (h.at + Duration::from_secs_f64(i as f64 / h.rate), f64::from(*x))))
        .filter(|(t, _)| *t >= from)
        .collect()
}

#[test]
fn what_is_heard_follows_the_clock_once_the_start_settles() {
    take();
    let (queue, clock) = (timed_queue(), Arc::new(Clock::default()));
    let start = Instant::now();
    clock.set(Some(Anchor { at: start, pos: 0.0, rate: 1.0 }));
    let thread = play(&queue, &clock);
    std::thread::sleep(Duration::from_millis(1500));
    let anchor = clock.get().unwrap();
    queue.lock().stop = true;
    clock.set(None);
    thread.join().unwrap();
    let heard = heard_after(&take(), start + Duration::from_secs(1));
    assert!(heard.len() > RATE as usize / 4, "heard {} frames", heard.len());
    let worst = heard.iter().map(|(t, media)| (anchor.position(*t) - media).abs()).fold(0.0, f64::max);
    // The sound only moves the clock once it runs a nudge's worth off, so that much may stay.
    assert!(worst < NUDGE + 0.002, "what was heard ran up to {:.2} ms off the clock", worst * 1000.0);
    assert!(anchor.pos != 0.0, "the start's latency error never moved the clock");
}

#[test]
fn a_pause_stops_the_sound_and_asks_for_what_the_device_dropped() {
    take();
    let (queue, clock) = (timed_queue(), Arc::new(Clock::default()));
    clock.set(Some(Anchor { at: Instant::now(), pos: 0.0, rate: 1.0 }));
    let thread = play(&queue, &clock);
    std::thread::sleep(Duration::from_millis(500));
    let at = clock.get().unwrap().position(Instant::now());
    clock.set(None);
    let paused = Instant::now();
    std::thread::sleep(Duration::from_millis(300));
    let reread = queue.lock().seek;
    queue.lock().stop = true;
    clock.changed.notify_all();
    thread.join().unwrap();
    assert!(take().iter().all(|h| h.at < paused + Duration::from_millis(60)), "sound played on after the pause");
    assert!(reread.is_some_and(|r| (r - (at - REREAD)).abs() < 0.03), "asked to read again from {reread:?}, paused at {at:.3}");
}
