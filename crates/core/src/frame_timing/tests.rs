use super::*;

fn queue(entries: &[(u64, Hns)]) -> VecDeque<(u64, Hns)> {
    entries.iter().copied().collect()
}

#[test]
fn exact_pts_pops_its_tick() {
    let mut q = queue(&[(0, 0), (1, 100), (2, 200)]);
    assert_eq!(match_tick(&mut q, 0, true), Some(0));
    assert_eq!(match_tick(&mut q, 100, false), Some(1));
    assert_eq!(q.len(), 1);
}

#[test]
fn lost_outputs_are_skipped_by_pts() {
    let mut q = queue(&[(0, 0), (1, 100), (2, 200), (3, 300)]);
    assert_eq!(match_tick(&mut q, 200, false), Some(2));
    assert_eq!(match_tick(&mut q, 300, false), Some(3));
    assert!(q.is_empty());
}

#[test]
fn missing_pts_after_the_first_falls_back_to_fifo() {
    let mut q = queue(&[(4, 400), (5, 500)]);
    assert_eq!(match_tick(&mut q, 0, false), Some(4));
    assert_eq!(match_tick(&mut q, 0, false), Some(5));
    assert_eq!(match_tick(&mut q, 0, false), None);
}

#[test]
fn zero_pts_on_the_first_output_matches_tick_zero() {
    let mut q = queue(&[(0, 0), (1, 100)]);
    assert_eq!(match_tick(&mut q, 0, true), Some(0));
}

#[test]
fn unknown_pts_matches_nothing() {
    let mut q = queue(&[(1, 100), (2, 200)]);
    assert_eq!(match_tick(&mut q, 150, false), None);
    assert_eq!(q.len(), 1);
    assert_eq!(match_tick(&mut q, 500, false), None);
    assert!(q.is_empty());
}

#[test]
fn deadlines_follow_exact_rational_ticks() {
    let mut pacer = Pacer::new(1_000, 60);
    for _ in 0..3 {
        pacer.advance();
    }
    assert_eq!(pacer.deadline(), 1_000 + 500_000);
    assert_eq!(pacer.pts(), 500_000);
    assert_eq!(pacer.frame_hns(), 166_666);
}

#[test]
fn first_tick_never_resyncs() {
    let mut pacer = Pacer::new(0, 60);
    assert!(!pacer.resync(10 * HNS_PER_SEC));
    assert_eq!(pacer.tick(), 0);
    pacer.advance();
    assert!(pacer.resync(10 * HNS_PER_SEC));
}

#[test]
fn resync_skips_to_real_time_only_when_far_behind() {
    let mut pacer = Pacer::new(0, 60);
    pacer.advance();
    assert!(!pacer.resync(2 * pacer.frame_hns()));
    assert!(pacer.resync(HNS_PER_SEC));
    assert_eq!(pacer.pts(), HNS_PER_SEC);
}

#[test]
fn stalls_after_five_seconds_without_output_while_submitting() {
    let mut dog = Watchdog::new(0);
    assert!(!dog.stalled(4 * HNS_PER_SEC, 0, true));
    assert!(dog.stalled(5 * HNS_PER_SEC, 0, true));
}

#[test]
fn output_resets_the_window() {
    let mut dog = Watchdog::new(0);
    assert!(!dog.stalled(4 * HNS_PER_SEC, 1, true));
    assert!(!dog.stalled(8 * HNS_PER_SEC, 1, true));
    assert!(dog.stalled(9 * HNS_PER_SEC, 1, true));
}

#[test]
fn idle_capture_never_stalls() {
    let mut dog = Watchdog::new(0);
    assert!(!dog.stalled(30 * HNS_PER_SEC, 0, false));
    assert!(!dog.stalled(34 * HNS_PER_SEC, 0, true));
    assert!(dog.stalled(35 * HNS_PER_SEC, 0, true));
}

fn pending(count: u64) -> VecDeque<Pending> {
    (0..count)
        .map(|i| Pending {
            tick: i,
            pts: i as Hns * 100,
            slot: i as usize,
        })
        .collect()
}

#[test]
fn counts_inputs_an_output_skips_past() {
    let q = queue(&[(0, 0), (1, 100), (2, 200), (3, 300)]);
    assert_eq!(count_skipped(&q, 0, true), 0);
    assert_eq!(count_skipped(&q, 100, false), 1);
    assert_eq!(count_skipped(&q, 250, false), 3);
    assert_eq!(count_skipped(&q, 0, false), 0);
}

#[test]
fn rising_output_releases_every_older_slot() {
    let mut rule = SlotRelease::new();
    let mut p = pending(5);
    assert_eq!(rule.on_output(&mut p, 0), vec![0]);
    assert_eq!(rule.on_output(&mut p, 300), vec![1, 2, 3]);
    assert_eq!(p.len(), 1);
    assert!(!rule.one_per_output());
}

#[test]
fn flush_releases_everything_once() {
    let mut p = pending(3);
    assert_eq!(SlotRelease::flush(&mut p), vec![0, 1, 2]);
    assert!(SlotRelease::flush(&mut p).is_empty());
}

#[test]
fn a_step_back_switches_to_one_per_output_for_good() {
    let mut rule = SlotRelease::new();
    let mut p = pending(10);
    assert_eq!(rule.on_output(&mut p, 0), vec![0]);
    assert_eq!(rule.on_output(&mut p, 300), vec![1, 2, 3]);
    assert_eq!(rule.on_output(&mut p, 100), vec![4]);
    assert!(rule.one_per_output());
    assert_eq!(rule.on_output(&mut p, 200), vec![5]);
    assert_eq!(rule.on_output(&mut p, 900), vec![6]);
    assert_eq!(rule.on_output(&mut p, 900), vec![7]);
}

#[test]
fn a_duplicate_output_releases_one_slot_not_more() {
    let mut rule = SlotRelease::new();
    let mut p = pending(4);
    assert_eq!(rule.on_output(&mut p, 100), vec![0, 1]);
    assert_eq!(rule.on_output(&mut p, 100), vec![2]);
    assert_eq!(p.len(), 1);
}

#[test]
fn an_encoder_that_swallows_inputs_never_exhausts_the_pool() {
    const SLOTS: usize = 6;
    const LAG: usize = 2;
    let mut free: Vec<usize> = (0..SLOTS).collect();
    let mut p: VecDeque<Pending> = VecDeque::new();
    let mut rule = SlotRelease::new();
    let mut outputs: VecDeque<Hns> = VecDeque::new();
    let mut encoded = 0;
    for frame in 0..2000u64 {
        let slot = free.pop().expect("pool ran out");
        let pts = frame as Hns * 100;
        p.push_back(Pending {
            tick: frame,
            pts,
            slot,
        });
        if frame % 7 != 3 {
            outputs.push_back(pts);
        }
        if frame == 1000 {
            free.extend(SlotRelease::flush(&mut p));
            outputs.clear();
        }
        if outputs.len() > LAG {
            let out = outputs.pop_front().unwrap();
            free.extend(rule.on_output(&mut p, out));
            encoded += 1;
        }
        assert!(free.len() + p.len() == SLOTS);
    }
    assert!(encoded > 1700);
    assert!(!rule.one_per_output());
}

#[test]
fn no_output_at_all_still_stalls_near_five_seconds() {
    let mut dog = Watchdog::new(0);
    let frame = HNS_PER_SEC / 60;
    let stalled_at = (1..1000).find(|&i| dog.stalled(i * frame, 0, true));
    assert!(matches!(stalled_at, Some(300 | 301)));
}

mod properties {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn pacer_grid_never_moves_backwards(fps in 1u32..=240, steps in 1usize..500) {
            let mut pacer = Pacer::new(0, fps);
            let mut last = pacer.deadline();
            for _ in 0..steps {
                pacer.advance();
                prop_assert!(pacer.deadline() >= last);
                prop_assert_eq!(pacer.deadline(), pacer.pts());
                last = pacer.deadline();
            }
        }

        #[test]
        fn resync_never_rewinds_and_lands_near_now(
            fps in 1u32..=240,
            start in 1u64..1000,
            now in 0i64..(600 * HNS_PER_SEC),
        ) {
            let mut pacer = Pacer::new(0, fps);
            for _ in 0..start {
                pacer.advance();
            }
            let before = pacer.tick();
            if pacer.resync(now) {
                prop_assert!(pacer.tick() >= before);
                prop_assert!(pacer.deadline() <= now);
            } else {
                prop_assert_eq!(pacer.tick(), before);
            }
        }

        #[test]
        fn output_ticks_strictly_rise(
            outputs in proptest::collection::vec((0i64..2000, any::<bool>()), 1..100),
        ) {
            let mut in_flight: VecDeque<(u64, Hns)> = (0..50).map(|i| (i, i as Hns * 40)).collect();
            let mut last = None;
            for (index, (pts, first)) in outputs.into_iter().enumerate() {
                let tick = next_tick(&mut in_flight, last, pts, first && index == 0);
                if let Some(previous) = last {
                    prop_assert!(tick > previous);
                }
                last = Some(tick);
            }
        }

        #[test]
        fn pts_is_the_exact_grid_whatever_the_wake_delays(
            fps in 24u32..=240,
            t0 in 0i64..HNS_PER_SEC,
            delays in proptest::collection::vec(0u64..=3, 10_000),
        ) {
            let mut pacer = Pacer::new(t0, fps);
            let frame = HNS_PER_SEC as f64 / f64::from(fps);
            let mut now = t0;
            for delay in delays {
                prop_assert_eq!(
                    pacer.pts(),
                    (u128::from(pacer.tick()) * HNS_PER_SEC as u128 / u128::from(fps)) as Hns
                );
                prop_assert_eq!(pacer.deadline(), t0 + pacer.pts());
                now = now.max(pacer.deadline()) + (delay as f64 * frame) as Hns;
                let before = pacer.tick();
                pacer.resync(now);
                prop_assert!(pacer.tick() >= before);
                pacer.advance();
            }
        }

        #[test]
        fn pts_stays_exact_at_huge_ticks(fps in 24u32..=240, tick in 0u64..(1u64 << 40)) {
            let mut pacer = Pacer::new(0, fps);
            pacer.tick = tick;
            let expected = (u128::from(tick) * HNS_PER_SEC as u128 / u128::from(fps)) as Hns;
            prop_assert_eq!(pacer.pts(), expected);
            pacer.advance();
            prop_assert!(pacer.pts() > expected);
        }

        #[test]
        fn the_wait_is_over_only_at_or_after_the_deadline(
            fps in 24u32..=240,
            ticks in 0u64..100_000,
            now in 0i64..(2000 * HNS_PER_SEC),
        ) {
            let mut pacer = Pacer::new(0, fps);
            pacer.tick = ticks;
            let deadline = pacer.deadline();
            prop_assert_eq!(pacer.remaining(now) <= 0, now >= deadline);
            if now < deadline {
                prop_assert_eq!(pacer.remaining(now), deadline - now);
            }
        }

        #[test]
        fn output_ticks_follow_the_pts_kinds(
            kinds in proptest::collection::vec(0u8..6, 1..120),
        ) {
            let mut in_flight: VecDeque<(u64, Hns)> =
                (0..200).map(|i| (i, 40 * (i as Hns + 1))).collect();
            let mut next_input = 0u64;
            let mut last: Option<u64> = None;
            for (index, kind) in kinds.into_iter().enumerate() {
                let front = in_flight.front().copied();
                let (pts, exact) = match (kind, front) {
                    (0 | 1, Some((tick, pts))) => (pts, Some(tick)),
                    (2, Some((_, pts))) => (pts + 1, None),
                    (3, Some((_, pts))) => (pts - 1, None),
                    (4, _) => (0, None),
                    (_, Some((_, pts))) => (pts.saturating_sub(40 * (next_input as Hns)), None),
                    (_, None) => (0, None),
                };
                next_input += 1;
                let tick = next_tick(&mut in_flight, last, pts, false);
                if let Some(previous) = last {
                    prop_assert!(tick > previous, "step {}: {} after {}", index, tick, previous);
                }
                if let (Some(exact), None) = (exact, last) {
                    prop_assert_eq!(tick, exact);
                }
                if let (Some(exact), Some(previous)) = (exact, last) {
                    prop_assert_eq!(tick, exact.max(previous + 1));
                }
                last = Some(tick);
            }
        }

        #[test]
        fn lost_outputs_fall_back_in_order(count in 1usize..200) {
            let mut in_flight: VecDeque<(u64, Hns)> = VecDeque::new();
            let mut last = None;
            for expected in 0..count as u64 {
                let tick = next_tick(&mut in_flight, last, 12_345, false);
                prop_assert_eq!(tick, expected);
                last = Some(tick);
            }
        }

        #[test]
        fn watchdog_only_stalls_while_submitting_and_idle_long_enough(
            gap in 0i64..(20 * HNS_PER_SEC),
            submitting in any::<bool>(),
        ) {
            let mut dog = Watchdog::new(0);
            let stalled = dog.stalled(gap, 0, submitting);
            prop_assert_eq!(stalled, submitting && gap >= 5 * HNS_PER_SEC);
        }
    }
}

fn decode_order_pts(frame_hns: Hns, b_frames: u64, groups: u64) -> Vec<Hns> {
    let mut order = vec![0u64];
    for group in 0..groups {
        let anchor = (group + 1) * (b_frames + 1);
        order.push(anchor);
        order.extend(anchor - b_frames..anchor);
    }
    order.into_iter().map(|n| n as Hns * frame_hns).collect()
}

fn frame_hns_at(fps_num: i64, fps_den: i64) -> Hns {
    HNS_PER_SEC * fps_den / fps_num
}

#[test]
fn strictly_rising_pts_pass_at_both_frame_rates() {
    for (num, den) in [(60, 1), (60_000, 1001)] {
        let step = frame_hns_at(num, den);
        let mut guard = OrderGuard::new();
        for n in 0..600 {
            assert_eq!(guard.check(n * step), Ok(()));
        }
        assert_eq!(guard.step_backs(), 0);
    }
}

#[test]
fn reordered_output_reports_a_step_back_for_one_to_three_b_frames() {
    for (num, den) in [(60, 1), (60_000, 1001)] {
        let step = frame_hns_at(num, den);
        for b_frames in 1..=3u64 {
            let pts = decode_order_pts(step, b_frames, 4);
            let mut guard = OrderGuard::new();
            let first_error = pts.iter().map(|&p| guard.check(p)).find_map(Result::err);
            let mut guard = OrderGuard::new();
            pts.iter().for_each(|&p| drop(guard.check(p)));
            let anchor = (b_frames + 1) as Hns * step;
            assert_eq!(
                first_error,
                Some(StepBack {
                    previous: anchor,
                    pts: anchor - b_frames as Hns * step,
                }),
                "{b_frames} B-frames at {num}/{den}"
            );
            assert_eq!(guard.step_backs(), 4, "{b_frames} B-frames");
        }
    }
}

#[test]
fn a_repeated_pts_is_a_step_back() {
    let mut guard = OrderGuard::new();
    assert_eq!(guard.check(100), Ok(()));
    assert_eq!(
        guard.check(100),
        Err(StepBack {
            previous: 100,
            pts: 100
        })
    );
}
