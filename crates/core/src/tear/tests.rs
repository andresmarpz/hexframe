use std::cell::Cell;
use std::time::Duration;

use super::*;

const CELLS: usize = 48 * 27;

fn scene(level: u8) -> Vec<u8> {
    (0..CELLS)
        .map(|i| level.wrapping_add((i % 7) as u8))
        .collect()
}

fn torn_from(base: &[u8], cells: usize) -> Vec<u8> {
    let mut out = base.to_vec();
    for value in out.iter_mut().take(cells) {
        *value = value.wrapping_add(120);
    }
    out
}

fn arrive(guard: &mut HoldBack, thumb: Vec<u8>) -> (usize, Arrival) {
    let slot = guard.write_slot();
    *guard.thumb_mut(slot) = thumb;
    (slot, guard.arrive(slot))
}

#[test]
fn a_frame_is_published_once_its_successor_arrives() {
    let mut guard = HoldBack::default();
    let (first, arrival) = arrive(&mut guard, scene(10));
    assert_eq!(arrival, Arrival::Held);
    assert_eq!(guard.published(), None);
    let (_, arrival) = arrive(&mut guard, scene(10));
    assert_eq!(arrival, Arrival::Published(first));
    assert_eq!(guard.published(), Some(first));
}

#[test]
fn a_frame_unlike_both_matching_neighbours_is_dropped() {
    let mut guard = HoldBack::default();
    let clean = scene(30);
    arrive(&mut guard, clean.clone());
    let (good, _) = arrive(&mut guard, clean.clone());
    let (torn, arrival) = arrive(&mut guard, torn_from(&clean, 300));
    assert_eq!(arrival, Arrival::Published(good));
    let (next, arrival) = arrive(&mut guard, clean.clone());
    assert_eq!(arrival, Arrival::Dropped(torn));
    assert_eq!(guard.published(), Some(good));
    assert_eq!(guard.pending(), Some(next));
}

#[test]
fn a_scene_cut_is_kept_because_the_next_frame_matches_it() {
    let mut guard = HoldBack::default();
    arrive(&mut guard, scene(30));
    arrive(&mut guard, scene(30));
    let (cut, _) = arrive(&mut guard, scene(200));
    let (_, arrival) = arrive(&mut guard, scene(200));
    assert_eq!(arrival, Arrival::Published(cut));
}

#[test]
fn steady_motion_is_never_dropped() {
    let mut guard = HoldBack::default();
    let mut dropped = 0;
    for step in 0..60u8 {
        let (_, arrival) = arrive(&mut guard, scene(step.wrapping_mul(9)));
        dropped += usize::from(matches!(arrival, Arrival::Dropped(_)));
    }
    assert_eq!(dropped, 0);
}

#[test]
fn a_small_local_change_is_not_a_tear() {
    let mut guard = HoldBack::default();
    let clean = scene(30);
    arrive(&mut guard, clean.clone());
    arrive(&mut guard, clean.clone());
    let (flash, _) = arrive(&mut guard, torn_from(&clean, 10));
    let (_, arrival) = arrive(&mut guard, clean);
    assert_eq!(arrival, Arrival::Published(flash));
}

#[test]
fn thumbnails_of_different_sizes_are_never_judged() {
    let mut guard = HoldBack::default();
    arrive(&mut guard, scene(30));
    arrive(&mut guard, scene(30));
    let (odd, _) = arrive(&mut guard, vec![0; 10]);
    let (_, arrival) = arrive(&mut guard, scene(30));
    assert_eq!(arrival, Arrival::Published(odd));
}

#[test]
fn the_write_slot_is_never_the_published_or_pending_one() {
    let mut guard = HoldBack::default();
    for step in 0..20u8 {
        let slot = guard.write_slot();
        assert_ne!(Some(slot), guard.published());
        assert_ne!(Some(slot), guard.pending());
        *guard.thumb_mut(slot) = scene(step);
        guard.arrive(slot);
    }
}

#[test]
fn promote_publishes_the_held_frame() {
    let mut guard = HoldBack::default();
    let (first, _) = arrive(&mut guard, scene(5));
    assert_eq!(guard.promote(), Some(first));
    assert_eq!(guard.published(), Some(first));
    assert_eq!(guard.pending(), None);
    assert_eq!(guard.promote(), None);
    let (_, arrival) = arrive(&mut guard, scene(5));
    assert_eq!(arrival, Arrival::Held);
}

#[test]
fn publish_now_skips_the_check() {
    let mut guard = HoldBack::default();
    arrive(&mut guard, scene(5));
    let slot = guard.write_slot();
    guard.publish_now(slot);
    assert_eq!(guard.published(), Some(slot));
    assert_eq!(guard.pending(), None);
}

#[test]
fn thumbnail_level_keeps_the_width_at_most_64() {
    assert_eq!(thumb_level((3072, 1728)), 6);
    assert_eq!(thumb_size((3072, 1728), 6), (48, 27));
    assert_eq!(thumb_level((1920, 1080)), 5);
    assert_eq!(thumb_size((1920, 1080), 5), (60, 33));
    assert_eq!(thumb_level((64, 36)), 0);
    assert_eq!(thumb_level((4096, 1)), 0);
}

#[test]
fn luma_spans_black_to_white() {
    assert_eq!(luma(0, 0, 0), 0);
    assert_eq!(luma(255, 255, 255), 255);
    assert!(luma(0, 255, 0) > luma(255, 0, 0));
}

struct FakeGpu {
    now: Cell<Duration>,
    done_at: Duration,
}

impl FakeGpu {
    fn wait(&self, budget: Duration) -> Wait {
        let result: Result<Wait, ()> = wait_bounded(
            budget,
            || self.now.get(),
            || Ok(self.now.get() >= self.done_at),
            |pause| {
                self.now
                    .set(self.now.get() + pause.max(Duration::from_micros(50)))
            },
        );
        result.unwrap_or(Wait::TimedOut(Duration::MAX))
    }
}

#[test]
fn the_wait_returns_as_soon_as_the_fence_signals() {
    let gpu = FakeGpu {
        now: Cell::new(Duration::ZERO),
        done_at: Duration::from_micros(800),
    };
    let Wait::Done(waited) = gpu.wait(COPY_WAIT_BUDGET) else {
        panic!("the copy finished inside the budget");
    };
    assert!(waited >= Duration::from_micros(800));
    assert!(waited < Duration::from_micros(1000));
}

#[test]
fn the_wait_gives_up_at_the_budget() {
    let gpu = FakeGpu {
        now: Cell::new(Duration::ZERO),
        done_at: Duration::from_secs(1),
    };
    let Wait::TimedOut(waited) = gpu.wait(COPY_WAIT_BUDGET) else {
        panic!("the copy never finished");
    };
    assert!(waited >= COPY_WAIT_BUDGET);
    assert!(waited < COPY_WAIT_BUDGET + Duration::from_millis(1));
}

#[test]
fn the_wait_passes_fence_errors_through() {
    let result = wait_bounded(
        COPY_WAIT_BUDGET,
        || Duration::ZERO,
        || Err::<bool, &str>("device removed"),
        |_| {},
    );
    assert_eq!(result, Err("device removed"));
}

#[test]
fn copy_waits_count_slow_and_timed_out_copies() {
    let mut waits = CopyWaits::default();
    waits.record(Wait::Done(Duration::from_micros(300)));
    waits.record(Wait::Done(COPY_WAIT_SLOW));
    waits.record(Wait::TimedOut(COPY_WAIT_BUDGET));
    assert_eq!(waits.slow, 1);
    assert_eq!(waits.timeouts, 1);
    assert_eq!(waits.max_us, 12_000);
}
