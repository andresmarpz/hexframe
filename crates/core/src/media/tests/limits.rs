use super::*;
use crate::media::reference::{audio_frames_to_ns, mul_div64_split, mul_div64_wide};
use proptest::prelude::*;

const SEC: Hns = 10_000_000;
const HOUR: Hns = 3600 * SEC;

fn frames_at(rate: u32, ms: i64) -> usize {
    (ms * i64::from(rate) / 1000) as usize
}

fn silence_spans(spans: &[Span]) -> Vec<usize> {
    spans
        .iter()
        .filter_map(|span| match *span {
            Span::Silence(n) => Some(n),
            Span::Data { .. } => None,
        })
        .collect()
}

fn data_frames(spans: &[Span]) -> usize {
    spans
        .iter()
        .map(|span| match *span {
            Span::Data { take, .. } => take,
            Span::Silence(_) => 0,
        })
        .sum()
}

fn at(rate: u32, t0: Hns) -> AudioTimeline {
    AudioTimeline::new(t0, rate).unwrap()
}

#[derive(Debug, Clone)]
enum Step {
    Packet {
        ts: Option<Hns>,
        now_step: Hns,
        frames: usize,
        silent: bool,
    },
    Timeout {
        now_step: Hns,
    },
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (
            prop::option::of(-(2 * HOUR)..(2 * HOUR)),
            0..(2 * SEC),
            0usize..5_000,
            any::<bool>()
        )
            .prop_map(|(ts, now_step, frames, silent)| Step::Packet {
                ts,
                now_step,
                frames,
                silent
            }),
        (0..(3 * SEC)).prop_map(|now_step| Step::Timeout { now_step }),
    ]
}

fn run_steps(rate: u32, start_frames: u64, t0: Hns, steps: &[Step]) {
    let mut timeline = at(rate, t0).with_frames_out(start_frames);
    let mut now = t0;
    let mut last = timeline.frames_out();
    let chunk = timeline.frames_for_hns(AudioTimeline::MAX_SILENCE_CHUNK);
    for step in steps {
        match *step {
            Step::Packet {
                ts,
                now_step,
                frames,
                silent,
            } => {
                now += now_step;
                let before = timeline.expected();
                let spans = timeline.on_packet_now(ts, now, frames, silent);
                let forward = ts.unwrap_or(before).max(now).saturating_sub(before).max(0);
                let bound = frames + timeline.frames_for_hns(forward) + 1;
                assert!(total(&spans) <= bound);
                assert!(silence_spans(&spans).iter().all(|n| *n <= chunk));
                for span in &spans {
                    if let Span::Data { skip, take } = *span {
                        assert!(skip + take <= frames);
                    }
                }
            }
            Step::Timeout { now_step } => {
                now += now_step;
                timeline.on_timeout(now);
            }
        }
        assert!(timeline.frames_out() >= last);
        last = timeline.frames_out();
    }
}

proptest! {
    #[test]
    fn arbitrary_input_never_panics_or_goes_backwards(
        rate in prop_oneof![Just(44_100u32), Just(48_000u32)],
        start in prop_oneof![Just(0u64), Just(1u64 << 31), Just(1u64 << 62)],
        t0 in 0..HOUR,
        steps in prop::collection::vec(step(), 1..40),
    ) {
        run_steps(rate, start, t0, &steps);
    }

    #[test]
    fn frames_and_hns_round_trip_within_one_frame(
        rate in prop_oneof![Just(44_100u32), Just(48_000u32)],
        start in prop_oneof![Just(0u64), Just(1u64 << 31), Just(1u64 << 40)],
        extra in 0u64..10_000_000,
    ) {
        let timeline = at(rate, 0);
        let frames = start + extra;
        let back = timeline.frames_for_hns(timeline.hns_for_frames(frames)) as u64;
        prop_assert!(back <= frames && frames - back <= 1);
    }

    #[test]
    fn conversion_matches_the_wide_reference(
        rate in prop_oneof![Just(44_100u32), Just(48_000u32)],
        frames in prop_oneof![0u64..(1 << 33), (1u64 << 40)..(1u64 << 48)],
    ) {
        let timeline = at(rate, 0);
        let expected_hns = audio_frames_to_ns(u64::from(rate), frames) / 100;
        let hns = timeline.hns_for_frames(frames);
        if expected_hns <= Hns::MAX as u64 {
            prop_assert!((hns as u64).abs_diff(expected_hns) <= 1);
        } else {
            prop_assert_eq!(hns, Hns::MAX);
        }
    }

    #[test]
    fn jitter_inside_the_deadband_is_untouched(
        jitter_ms in -20i64..=20,
        len_ms in 1i64..40,
    ) {
        let mut timeline = timeline(0);
        timeline.on_packet(Some(0), frames(10), false);
        let spans = timeline.on_packet(Some(10 * MS + jitter_ms * MS), frames(len_ms), false);
        prop_assert_eq!(spans, vec![Span::Data { skip: 0, take: frames(len_ms) }]);
    }

    #[test]
    fn first_packet_offset_never_moves_frames_back(
        offset_ms in 0i64..1000,
        now_ms in 0i64..2000,
        len_ms in 1i64..40,
    ) {
        let mut timeline = timeline(0);
        let spans = timeline.on_packet_now(Some(offset_ms * MS), now_ms * MS, frames(len_ms), false);
        prop_assert!(data_frames(&spans) <= frames(len_ms));
        prop_assert_eq!(timeline.frames_out() as usize, total(&spans));
        if (offset_ms - now_ms).abs() <= 500 {
            prop_assert!(total(&spans) >= frames(len_ms));
        }
    }
}

#[test]
fn split_formula_agrees_with_wide_when_nothing_overflows() {
    for frames in [0u64, 1, 47_999, 48_000, 1 << 20, 1 << 33] {
        assert_eq!(
            mul_div64_split(frames, 1_000_000_000, 48_000),
            mul_div64_wide(frames, 1_000_000_000, 48_000)
        );
    }
}

#[test]
fn zero_sample_rate_is_refused() {
    assert!(AudioTimeline::new(0, 0).is_none());
}

#[test]
fn packet_behind_and_resent_packet_table() {
    let mut timeline = timeline(0);
    timeline.on_packet(Some(0), frames(100), false);
    let behind = timeline.on_packet(Some(50 * MS), frames(100), false);
    assert_eq!(
        behind,
        vec![Span::Data {
            skip: frames(50),
            take: frames(50)
        }]
    );
    let resent = timeline.on_packet(Some(50 * MS), frames(100), false);
    assert!(resent.is_empty());
}

#[test]
fn two_seconds_behind_trims_until_under_the_deadband() {
    let mut timeline = timeline(0);
    timeline.on_packet(Some(0), frames(2_000), false);
    let spans = timeline.on_packet(Some(0), frames(2_020), false);
    assert_eq!(
        spans,
        vec![Span::Data {
            skip: frames(2_000),
            take: frames(20)
        }]
    );
}

#[test]
fn forward_then_back_never_rewinds_emitted_audio() {
    let mut timeline = timeline(0);
    timeline.on_packet(Some(0), frames(10), false);
    timeline.on_packet(Some(5_000 * MS), frames(10), false);
    let out = timeline.frames_out();
    timeline.on_packet(Some(20 * MS), frames(10), false);
    assert!(timeline.frames_out() >= out);
}

#[test]
fn first_packet_without_timestamp_appends() {
    let mut timeline = timeline(0);
    let spans = timeline.on_packet(None, frames(10), false);
    assert_eq!(
        spans,
        vec![Span::Data {
            skip: 0,
            take: frames(10)
        }]
    );
}

#[test]
fn packet_stamped_90_ms_before_a_timeout_loses_nothing() {
    let mut timeline = timeline(0);
    timeline.on_timeout(SEC);
    let spans = timeline.on_packet(Some(SEC - 90 * MS), frames(10), false);
    assert_eq!(data_frames(&spans), frames(10));
}

#[test]
fn packet_stamped_200_ms_before_a_timeout_loses_its_first_140_ms() {
    let mut timeline = timeline(0);
    timeline.on_timeout(SEC);
    let spans = timeline.on_packet(Some(SEC - 200 * MS), frames(300), false);
    assert_eq!(
        spans,
        vec![Span::Data {
            skip: frames(140),
            take: frames(160)
        }]
    );
}

#[test]
fn single_frame_packets_stay_gapless() {
    let mut timeline = timeline(0);
    let mut out = 0;
    for i in 0..4_800i64 {
        let ts = i * SEC / 48_000;
        out += total(&timeline.on_packet(Some(ts), 1, false));
    }
    assert_eq!(out, 4_800);
    assert_eq!(timeline.big_gaps(), 0);
}

#[test]
fn restart_length_equals_wall_time() {
    let mut timeline = timeline(5 * SEC);
    for i in 0..100i64 {
        timeline.on_packet(Some(5 * SEC + i * 10 * MS), frames(10), false);
    }
    assert_eq!(timeline.frames_out(), frames(1_000) as u64);
}

#[test]
fn one_hour_forward_jump_is_chunked_and_counted() {
    for rate in [44_100u32, 48_000] {
        let mut timeline = at(rate, 0);
        timeline.on_packet(Some(0), frames_at(rate, 10), false);
        let gap_start = timeline.expected();
        let spans = timeline.on_packet(Some(gap_start + HOUR), frames_at(rate, 10), false);
        let silence = silence_spans(&spans);
        assert!(silence.iter().all(|n| *n <= timeline.frames_for_hns(SEC)));
        assert_eq!(silence.iter().sum::<usize>(), timeline.frames_for_hns(HOUR));
        assert_eq!(data_frames(&spans), frames_at(rate, 10));
        assert_eq!(timeline.largest_gap(), HOUR);
        assert_eq!(timeline.big_gaps(), 1);
    }
}

#[test]
fn one_hour_backward_jump_is_counted_and_never_rewinds() {
    let mut timeline = timeline(HOUR);
    timeline.on_packet(Some(HOUR), frames(10), false);
    let out = timeline.frames_out();
    let spans = timeline.on_packet(Some(0), frames(10), false);
    assert!(spans.is_empty());
    assert_eq!(timeline.frames_out(), out);
    assert!(timeline.largest_gap() >= HOUR - 20 * MS);
    assert_eq!(timeline.big_gaps(), 1);
    for i in 1..400i64 {
        timeline.on_packet(Some(i * 10 * MS), frames(10), false);
    }
    assert_eq!(timeline.big_gaps(), 1);
}

#[test]
fn forward_jump_then_normal_packets_is_one_count() {
    let mut timeline = timeline(0);
    timeline.on_packet(Some(0), frames(10), false);
    let jumped = timeline.expected() + HOUR;
    timeline.on_packet(Some(jumped), frames(10), false);
    for _ in 0..400 {
        let ts = timeline.expected();
        timeline.on_packet(Some(ts), frames(10), false);
    }
    assert_eq!(timeline.big_gaps(), 1);
}

#[test]
fn forward_jump_then_back_is_one_count_per_jump() {
    let mut timeline = timeline(0);
    timeline.on_packet(Some(0), frames(10), false);
    let jumped = timeline.expected() + HOUR;
    timeline.on_packet(Some(jumped), frames(10), false);
    for i in 1..400i64 {
        timeline.on_packet(Some(i * 10 * MS), frames(10), false);
    }
    assert_eq!(timeline.big_gaps(), 2);
}

#[test]
fn first_packet_is_padded_by_its_exact_offset() {
    for rate in [44_100u32, 48_000] {
        for offset_ms in [0i64, 5, 20, 29, 40] {
            let mut timeline = AudioTimeline::new(0, rate).expect("rate");
            let len = frames_at(rate, 10);
            let spans = timeline.on_packet(Some(offset_ms * MS), len, false);
            let silence: usize = silence_spans(&spans).iter().sum();
            let expected = frames_at(rate, offset_ms);
            assert!(
                silence.abs_diff(expected) <= 1,
                "{rate} Hz {offset_ms} ms: {silence} != {expected}"
            );
            assert_eq!(data_frames(&spans), len);
        }
    }
}

#[test]
fn later_packets_keep_the_deadband() {
    let mut timeline = AudioTimeline::new(0, 48_000).expect("rate");
    let len = frames_at(48_000, 10);
    timeline.on_packet(Some(0), len, false);
    let next = timeline.on_packet(Some(30 * MS), len, false);
    assert!(silence_spans(&next).is_empty());
}
