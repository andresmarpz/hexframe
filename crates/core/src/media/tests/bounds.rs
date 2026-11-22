use super::*;
use crate::time::HNS_PER_SEC;
use proptest::prelude::*;

const SEC: Hns = HNS_PER_SEC;
const HOUR: Hns = 3600 * SEC;

fn at(rate: u32, t0: Hns) -> AudioTimeline {
    AudioTimeline::new(t0, rate).unwrap()
}

proptest! {
    #[test]
    fn capture_interval_stays_within_one_output_frame(fps in 1u32..=240, hz in 24u32..=800) {
        let interval = capture_min_update_interval_hns(fps, hz);
        let frame = HNS_PER_SEC / i64::from(fps);
        prop_assert!(interval >= 0);
        prop_assert!(interval <= frame);
        prop_assert!(interval <= HNS_PER_SEC * 9 / 10 / i64::from(fps));
        let refresh = HNS_PER_SEC / i64::from(hz);
        if refresh + 5_000 < frame {
            prop_assert!(interval > 0);
        }
    }

    #[test]
    fn skipped_audio_never_exceeds_its_packet(
        rate in prop_oneof![Just(44_100u32), Just(48_000u32)],
        t0 in 0..HOUR,
        early in 0..AudioTimeline::FIRST_DISTRUST,
        far_early in 0..(3 * HOUR),
        back in 0..(10 * SEC),
        frames in 1usize..5_000,
        trusted in any::<bool>(),
    ) {
        let mut timeline = at(rate, t0);
        let (first_ts, now) = if trusted {
            (t0 - early, t0)
        } else {
            (t0 - far_early, t0 + 20 * SEC)
        };
        let first = timeline.on_packet_now(Some(first_ts), now, frames, false);
        assert_skips_stay_inside(&first, frames)?;
        let second_ts = timeline.expected() - back;
        let second = timeline.on_packet_now(Some(second_ts), now, frames, false);
        assert_skips_stay_inside(&second, frames)?;
    }
}

fn assert_skips_stay_inside(spans: &[Span], frames: usize) -> Result<(), TestCaseError> {
    for span in spans {
        if let Span::Data { skip, take } = *span {
            prop_assert!(skip <= frames);
            prop_assert_eq!(skip + take, frames);
        }
    }
    Ok(())
}

#[test]
fn a_trusted_packet_far_behind_the_timeline_skips_exactly_its_own_frames() {
    let mut timeline = at(48_000, 100 * SEC);
    timeline.on_packet_now(Some(100 * SEC), 100 * SEC, 480, false);
    let ts = timeline.expected() - 5 * SEC;
    let spans = timeline.on_packet_now(Some(ts), 100 * SEC, 480, false);
    assert!(spans.is_empty());
    assert_eq!(timeline.frames_out(), 480);
    assert_eq!(timeline.trims(), 1);
}

#[test]
fn a_first_packet_ten_seconds_before_session_start_does_not_underflow() {
    let mut timeline = at(48_000, 100 * SEC);
    let spans = timeline.on_packet_now(Some(90 * SEC), 100 * SEC, 480, false);
    assert!(spans.is_empty());
    assert_eq!(timeline.frames_out(), 0);
}
