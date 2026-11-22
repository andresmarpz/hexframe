use std::cell::Cell;
use std::time::Duration;

use super::*;

mod model;

const WIDTH: usize = 48;
const CELLS: usize = WIDTH * 27;

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

fn store(guard: &mut HoldBack, slot: usize, mut thumb: Vec<u8>) {
    let width = if thumb.len().is_multiple_of(WIDTH) {
        WIDTH
    } else {
        1
    };
    guard.store_thumb(slot, &mut thumb, width);
}

fn arrive(guard: &mut HoldBack, thumb: Vec<u8>) -> (usize, Arrival) {
    let slot = guard.write_slot();
    store(guard, slot, thumb);
    (slot, guard.arrive(slot))
}

#[test]
fn a_frame_is_published_once_its_successor_arrives() {
    let mut guard = HoldBack::default();
    let (first, arrival) = arrive(&mut guard, scene(10));
    assert_eq!(arrival, Arrival::Held);
    assert_eq!(guard.published(), None);
    let (_, arrival) = arrive(&mut guard, scene(40));
    assert_eq!(arrival, Arrival::Published(first));
    assert_eq!(guard.published(), Some(first));
}

#[test]
fn a_frame_unlike_both_matching_neighbours_is_dropped() {
    let mut guard = HoldBack::default();
    let clean = scene(30);
    arrive(&mut guard, scene(10));
    let (good, _) = arrive(&mut guard, clean.clone());
    let (torn, arrival) = arrive(&mut guard, torn_from(&clean, 300));
    assert_eq!(arrival, Arrival::Published(good));
    let (next, arrival) = arrive(&mut guard, clean);
    assert_eq!(arrival, Arrival::Dropped(torn));
    assert_eq!(guard.published(), Some(next));
    assert_eq!(guard.pending(), None);
    assert_ne!(guard.write_slot(), next);
}

#[test]
fn the_frame_after_a_dropped_one_is_published_at_once() {
    let mut guard = HoldBack::default();
    let clean = scene(30);
    arrive(&mut guard, scene(10));
    arrive(&mut guard, clean.clone());
    let (torn, _) = arrive(&mut guard, torn_from(&clean, 300));
    let (next, arrival) = arrive(&mut guard, clean);
    assert_eq!(arrival, Arrival::Dropped(torn));
    assert_eq!(guard.published(), Some(next));
    assert_eq!(guard.promote(), None);
    let (following, arrival) = arrive(&mut guard, scene(60));
    assert_eq!(arrival, Arrival::Held);
    let (_, arrival) = arrive(&mut guard, scene(90));
    assert_eq!(arrival, Arrival::Published(following));
}

#[test]
fn a_scene_cut_is_kept_because_the_next_frame_matches_it() {
    let mut guard = HoldBack::default();
    arrive(&mut guard, scene(10));
    arrive(&mut guard, scene(30));
    arrive(&mut guard, scene(200));
    let (second, arrival) = arrive(&mut guard, scene(200));
    assert_eq!(
        arrival,
        Arrival::Merged {
            content_changed: true
        }
    );
    assert_eq!(guard.published(), Some(second));
    assert_eq!(guard.pending(), None);
}

#[test]
fn unchanged_and_cursor_only_frames_merge_without_taking_an_extra_slot() {
    let mut guard = HoldBack::default();
    let frame_a = world(0);
    let frame_b = world(3);
    let mut frame_b_cursor = frame_b.clone();
    // Move a 2x2 cursor spot on a 96x54 thumbnail (4 cells total).
    for i in [100, 101, 200, 201] {
        frame_b_cursor[i] = frame_b_cursor[i].wrapping_add(120);
    }
    let frame_c = world(6);

    let feed_one = |guard: &mut HoldBack, mut thumb: Vec<u8>| {
        let slot = guard.write_slot();
        guard.store_thumb(slot, &mut thumb, GAME_WIDTH);
        (slot, guard.arrive(slot))
    };

    let (a, arr) = feed_one(&mut guard, frame_a);
    assert_eq!(arr, Arrival::Held);
    let (_, arr) = feed_one(&mut guard, frame_b);
    assert_eq!(arr, Arrival::Published(a));
    let (b_cursor, arr) = feed_one(&mut guard, frame_b_cursor.clone());
    assert_eq!(
        arr,
        Arrival::Merged {
            content_changed: true
        }
    );
    assert_eq!(guard.published(), Some(b_cursor));
    assert_eq!(guard.pending(), None);

    // A second cursor-only frame updates published in place without changing content.
    let (b_cursor2, arr) = feed_one(&mut guard, frame_b_cursor);
    assert_eq!(
        arr,
        Arrival::Merged {
            content_changed: false
        }
    );
    assert_eq!(guard.published(), Some(b_cursor2));
    assert_eq!(guard.pending(), None);

    // The next real frame enters pending immediately without being delayed behind a duplicate.
    let (c, arr) = feed_one(&mut guard, frame_c);
    assert_eq!(arr, Arrival::Held);
    assert_eq!(guard.pending(), Some(c));
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
        store(&mut guard, slot, scene(step));
        guard.arrive(slot);
    }
}

#[test]
fn the_slot_being_read_survives_later_arrivals() {
    let mut guard = HoldBack::default();
    arrive(&mut guard, scene(1));
    arrive(&mut guard, scene(2));
    let read = guard.start_reading();
    assert!(read.is_some());
    for step in 3..12u8 {
        let (slot, _) = arrive(&mut guard, scene(step));
        assert_ne!(Some(slot), read);
    }
    guard.done_reading();
    assert_eq!(guard.reading(), None);
}

#[test]
fn the_write_slot_avoids_published_pending_and_read_slots_in_any_sequence() {
    let mut guard = HoldBack::default();
    let mut seed: u32 = 0x9e37_79b9;
    let mut drops = 0;
    for step in 0..5000u32 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let slot = guard.write_slot();
        let taken = [guard.published(), guard.pending(), guard.reading()];
        assert!(slot < RING_SLOTS);
        assert!(
            !taken.contains(&Some(slot)),
            "step {step}: {slot} in {taken:?}"
        );
        match seed % 8 {
            0 => {
                guard.promote();
            }
            1 => {
                guard.start_reading();
            }
            2 => guard.done_reading(),
            3 => guard.publish_now(slot),
            4 => {
                store(&mut guard, slot, torn_from(&scene(30), 300));
                guard.arrive(slot);
            }
            _ => {
                store(&mut guard, slot, scene(30));
                drops += usize::from(matches!(guard.arrive(slot), Arrival::Dropped(_)));
            }
        }
    }
    assert!(drops > 0);
}

#[test]
fn promote_publishes_the_held_frame() {
    let mut guard = HoldBack::default();
    let (first, _) = arrive(&mut guard, scene(5));
    assert_eq!(guard.promote(), Some(first));
    assert_eq!(guard.published(), Some(first));
    assert_eq!(guard.pending(), None);
    assert_eq!(guard.promote(), None);
    let (_, arrival) = arrive(&mut guard, scene(30));
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
fn thumbnail_level_keeps_the_width_at_most_128() {
    assert_eq!(thumb_level((3072, 1728)), 5);
    assert_eq!(thumb_size((3072, 1728), 5), (96, 54));
    assert_eq!(thumb_level((2560, 1440)), 5);
    assert_eq!(thumb_size((2560, 1440), 5), (80, 45));
    assert_eq!(thumb_level((1920, 1080)), 4);
    assert_eq!(thumb_size((1920, 1080), 4), (120, 67));
    assert_eq!(thumb_level((128, 72)), 0);
    assert_eq!(thumb_level((200, 2)), 1);
    assert_eq!(thumb_size((200, 2), 1), (100, 1));
    assert_eq!(thumb_level((64, 36)), 0);
    assert_eq!(thumb_level((4096, 1)), 0);
}

#[test]
fn thumbnails_of_different_widths_skip_the_tile_rule() {
    let mut guard = HoldBack::default();
    let clean = scene(30);
    arrive(&mut guard, clean.clone());
    arrive(&mut guard, clean.clone());
    let mut blink = clean.clone();
    blink[..8].fill(200);
    blink[WIDTH..WIDTH + 8].fill(200);
    let slot = guard.write_slot();
    guard.store_thumb(slot, &mut blink, WIDTH / 2);
    guard.arrive(slot);
    let (_, arrival) = arrive(&mut guard, clean);
    assert_eq!(arrival, Arrival::Published(slot));
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

const GAME_WIDTH: usize = 96;
const GAME_HEIGHT: usize = 54;
const HUD_COLS: std::ops::Range<usize> = 70..90;
const HUD_ROWS: std::ops::Range<usize> = 36..52;

fn noise(seed: usize) -> u8 {
    let mut x = (seed as u32).wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    x as u8
}

fn world(shift: usize) -> Vec<u8> {
    (0..GAME_WIDTH * GAME_HEIGHT)
        .map(|i| noise(i % GAME_WIDTH + shift + (i / GAME_WIDTH) * 1000))
        .collect()
}

fn with_hud(mut frame: Vec<u8>, hud: impl Fn(usize, usize) -> u8) -> Vec<u8> {
    for y in HUD_ROWS {
        for x in HUD_COLS {
            frame[y * GAME_WIDTH + x] = hud(x, y);
        }
    }
    frame
}

fn minimap(_: usize, _: usize) -> u8 {
    20
}

fn checkerboard(x: usize, y: usize) -> u8 {
    if (x + y).is_multiple_of(2) { 0 } else { 255 }
}

fn feed(frames: impl IntoIterator<Item = Vec<u8>>) -> Vec<usize> {
    let mut guard = HoldBack::default();
    let mut dropped = Vec::new();
    for (index, mut thumb) in frames.into_iter().enumerate() {
        let slot = guard.write_slot();
        guard.store_thumb(slot, &mut thumb, GAME_WIDTH);
        if let Arrival::Dropped(_) = guard.arrive(slot) {
            dropped.push(index - 1);
        }
    }
    dropped
}

fn game(frames: usize, hud_at: impl Fn(usize) -> Option<fn(usize, usize) -> u8>) -> Vec<Vec<u8>> {
    (0..frames)
        .map(|step| {
            let frame = world(step * 3);
            match hud_at(step) {
                Some(hud) => with_hud(frame, hud),
                None => frame,
            }
        })
        .collect()
}

#[test]
fn a_hud_missing_for_one_frame_while_the_world_moves_is_dropped() {
    let frames = game(12, |step| {
        (step != 6).then_some(minimap as fn(usize, usize) -> u8)
    });
    assert_eq!(feed(frames), vec![6]);
}

#[test]
fn a_hud_turned_to_a_checkerboard_for_one_frame_is_dropped() {
    let frames = game(12, |step| {
        Some(if step == 6 { checkerboard } else { minimap })
    });
    assert_eq!(feed(frames), vec![6]);
}

#[test]
fn a_camera_pan_with_no_hud_is_never_dropped() {
    assert_eq!(feed(game(60, |_| None)), Vec::<usize>::new());
}

#[test]
fn a_camera_pan_under_a_steady_hud_is_never_dropped() {
    assert_eq!(feed(game(60, |_| Some(minimap))), Vec::<usize>::new());
}

#[test]
fn a_hud_element_that_opens_and_stays_is_kept() {
    let frames = (0..12).map(|step| {
        let frame = world(step * 3);
        if step >= 6 {
            with_hud(frame, minimap)
        } else {
            frame
        }
    });
    assert_eq!(feed(frames), Vec::<usize>::new());
}

#[test]
fn a_hud_element_that_closes_and_stays_closed_is_kept() {
    let frames = (0..12).map(|step| {
        let frame = world(step * 3);
        if step < 6 {
            with_hud(frame, minimap)
        } else {
            frame
        }
    });
    assert_eq!(feed(frames), Vec::<usize>::new());
}

#[test]
fn a_one_frame_full_screen_flash_is_dropped() {
    let frames = (0..12).map(|step| {
        if step == 6 {
            vec![255; GAME_WIDTH * GAME_HEIGHT]
        } else {
            with_hud(scene_of(40), minimap)
        }
    });
    assert_eq!(feed(frames), vec![6]);
}

fn scene_of(level: u8) -> Vec<u8> {
    (0..GAME_WIDTH * GAME_HEIGHT)
        .map(|i| level.wrapping_add((i % 7) as u8))
        .collect()
}

#[test]
fn noise_below_the_cell_threshold_never_drops() {
    let base = with_hud(world(0), minimap);
    let frames = (0..200).map(|step| {
        base.iter()
            .enumerate()
            .map(|(i, &value)| {
                let jitter = i32::from(noise(step * 7919 + i) % 31) - 15;
                (i32::from(value) + jitter).clamp(0, 255) as u8
            })
            .collect()
    });
    assert_eq!(feed(frames), Vec::<usize>::new());
}

#[test]
fn a_still_world_with_a_hud_blink_is_dropped() {
    let still = with_hud(world(0), minimap);
    let frames = (0..12).map(|step| if step == 6 { world(0) } else { still.clone() });
    assert_eq!(feed(frames), vec![6]);
}

fn flash_at(rows: std::ops::Range<usize>, cols: std::ops::Range<usize>) -> Vec<usize> {
    let still = world(0);
    let frames = (0..12).map(|step| {
        let mut frame = still.clone();
        if step == 6 {
            for y in rows.clone() {
                frame[y * GAME_WIDTH + cols.start..y * GAME_WIDTH + cols.end].fill(255);
            }
        }
        frame
    });
    feed(frames)
}

#[test]
fn a_one_frame_flash_in_the_middle_of_the_screen_is_kept() {
    assert_eq!(flash_at(20..28, 40..48), Vec::<usize>::new());
    assert_eq!(flash_at(20..28, 8..16), Vec::<usize>::new());
}

#[test]
fn a_one_frame_flash_in_the_bottom_corners_outside_the_hud_is_kept() {
    assert_eq!(flash_at(40..48, 8..16), Vec::<usize>::new());
    assert_eq!(flash_at(8..16, 80..88), Vec::<usize>::new());
}

#[test]
fn a_one_frame_flash_over_the_minimap_is_dropped() {
    assert_eq!(flash_at(40..48, 80..88), vec![6]);
}

#[test]
fn a_one_frame_flash_over_the_ability_bar_is_dropped() {
    assert_eq!(flash_at(40..48, 40..48), vec![6]);
}

#[test]
fn a_one_frame_flash_over_the_score_corner_is_dropped() {
    assert_eq!(flash_at(0..8, 80..88), vec![6]);
}
