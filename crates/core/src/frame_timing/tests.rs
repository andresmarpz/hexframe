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
