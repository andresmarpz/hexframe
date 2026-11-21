use super::*;

#[test]
fn frame_ticks_at_60_fps_is_1500() {
    let format = VideoFormat {
        width: 1920,
        height: 1080,
        fps: 60,
        sps: Vec::new(),
        pps: Vec::new(),
    };
    assert_eq!(format.frame_ticks(), 1500);
}

const RATE: u32 = 48_000;
const MS: Hns = 10_000;

fn frames(ms: i64) -> usize {
    (ms * 48) as usize
}

fn total(spans: &[Span]) -> usize {
    spans
        .iter()
        .map(|span| match *span {
            Span::Silence(n) => n,
            Span::Data { take, .. } => take,
        })
        .sum()
}

#[test]
fn first_packet_pads_from_t0() {
    let mut timeline = AudioTimeline::new(1_000_000, RATE);
    let spans = timeline.on_packet(Some(1_000_000 + 100 * MS), frames(10), false);
    assert_eq!(
        spans,
        vec![
            Span::Silence(frames(100)),
            Span::Data {
                skip: 0,
                take: frames(10)
            }
        ]
    );
    assert_eq!(timeline.frames_out(), frames(110) as u64);
}

#[test]
fn gap_is_filled_with_silence() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_packet(Some(0), frames(10), false);
    let spans = timeline.on_packet(Some(60 * MS), frames(10), false);
    assert_eq!(spans[0], Span::Silence(frames(50)));
    assert_eq!(timeline.frames_out(), frames(70) as u64);
}

#[test]
fn overlap_drops_leading_frames() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_packet(Some(0), frames(10), false);
    let dropped = timeline.on_packet(Some(-50 * MS), frames(10), false);
    assert!(dropped.is_empty());
    assert_eq!(timeline.frames_out(), frames(10) as u64);
    let spans = timeline.on_packet(Some(-30 * MS), frames(100), false);
    assert_eq!(
        spans,
        vec![Span::Data {
            skip: frames(40),
            take: frames(60)
        }]
    );
    assert_eq!(timeline.frames_out(), frames(70) as u64);
}

#[test]
fn small_jitter_is_ignored() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_packet(Some(0), frames(10), false);
    let late = timeline.on_packet(Some(10 * MS + 20 * MS), frames(10), false);
    assert_eq!(
        late,
        vec![Span::Data {
            skip: 0,
            take: frames(10)
        }]
    );
    let early = timeline.on_packet(Some(20 * MS - 20 * MS), frames(10), false);
    assert_eq!(
        early,
        vec![Span::Data {
            skip: 0,
            take: frames(10)
        }]
    );
}

#[test]
fn silent_packet_becomes_silence() {
    let mut timeline = AudioTimeline::new(0, RATE);
    let spans = timeline.on_packet(Some(0), frames(10), true);
    assert_eq!(spans, vec![Span::Silence(frames(10))]);
}

#[test]
fn missing_timestamp_appends() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_packet(None, frames(10), false);
    let spans = timeline.on_packet(None, frames(10), false);
    assert_eq!(
        spans,
        vec![Span::Data {
            skip: 0,
            take: frames(10)
        }]
    );
    assert_eq!(timeline.frames_out(), frames(20) as u64);
}

#[test]
fn timeout_fills_up_to_now_minus_safety() {
    let mut timeline = AudioTimeline::new(0, RATE);
    assert_eq!(timeline.on_timeout(50 * MS), None);
    assert_eq!(
        timeline.on_timeout(160 * MS),
        Some(Span::Silence(frames(100)))
    );
    assert_eq!(timeline.on_timeout(160 * MS), None);
    assert_eq!(timeline.frames_out(), frames(100) as u64);
}

#[test]
fn packet_after_timeout_fill_overlaps_and_is_trimmed() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_timeout(160 * MS);
    let spans = timeline.on_packet(Some(40 * MS), frames(80), false);
    assert_eq!(
        spans,
        vec![Span::Data {
            skip: frames(60),
            take: frames(20)
        }]
    );
}

#[test]
fn distrusts_first_timestamp_far_from_now() {
    let mut timeline = AudioTimeline::new(0, RATE);
    let spans = timeline.on_packet_now(Some(10_000 * MS), 200 * MS, frames(10), false);
    assert_eq!(
        spans,
        vec![
            Span::Silence(frames(190)),
            Span::Data {
                skip: 0,
                take: frames(10)
            }
        ]
    );
    let next = timeline.on_packet_now(Some(99_999 * MS), 210 * MS, frames(10), false);
    assert_eq!(
        next,
        vec![Span::Data {
            skip: 0,
            take: frames(10)
        }]
    );
}

#[test]
fn distrusted_timestamps_reanchor_on_now_past_the_threshold() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_packet_now(Some(10_000 * MS), 200 * MS, frames(10), false);
    let small = timeline.on_packet_now(Some(1), 220 * MS, frames(10), false);
    assert_eq!(total(&small), frames(10));
    assert!(!small.iter().any(|s| matches!(s, Span::Silence(_))));
    let drifted = timeline.on_packet_now(Some(1), 280 * MS, frames(10), false);
    assert_eq!(
        drifted,
        vec![
            Span::Silence(frames(60)),
            Span::Data {
                skip: 0,
                take: frames(10)
            }
        ]
    );
}

#[test]
fn distrusted_timestamps_drop_frames_when_audio_runs_ahead() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_packet_now(Some(10_000 * MS), 200 * MS, frames(10), false);
    let spans = timeline.on_packet_now(None, 210 * MS, frames(10), false);
    assert_eq!(total(&spans), frames(10));
    let ahead = timeline.on_packet_now(None, 225 * MS, frames(50), false);
    assert_eq!(
        ahead,
        vec![Span::Data {
            skip: frames(35),
            take: frames(15)
        }]
    );
}

#[test]
fn distrusted_jitter_under_the_threshold_is_ignored() {
    let mut timeline = AudioTimeline::new(0, RATE);
    timeline.on_packet_now(Some(10_000 * MS), 200 * MS, frames(10), false);
    let spans = timeline.on_packet_now(None, 225 * MS, frames(10), false);
    assert_eq!(
        spans,
        vec![Span::Data {
            skip: 0,
            take: frames(10)
        }]
    );
}

#[test]
fn trusts_first_timestamp_near_now() {
    let mut timeline = AudioTimeline::new(0, RATE);
    let spans = timeline.on_packet_now(Some(100 * MS), 300 * MS, frames(10), false);
    assert_eq!(spans[0], Span::Silence(frames(100)));
}

#[test]
fn drift_over_many_packets_stays_gapless() {
    let mut timeline = AudioTimeline::new(0, RATE);
    let mut emitted = 0usize;
    for i in 0..2000i64 {
        let ts = i * 10 * MS + i * 50;
        emitted += total(&timeline.on_packet(Some(ts), frames(10), false));
    }
    assert_eq!(timeline.frames_out(), emitted as u64);
    let expected = 2000 * frames(10) as i64;
    assert!((emitted as i64 - expected).abs() <= frames(40) as i64 + 480 * 10);
}

#[test]
fn cumulative_frames_never_go_backwards() {
    let mut timeline = AudioTimeline::new(0, RATE);
    let mut last = 0;
    let stamps = [0, 500 * MS, 20 * MS, 600 * MS, 0, 1_000 * MS, 40 * MS];
    for ts in stamps {
        timeline.on_packet(Some(ts), frames(10), false);
        assert!(timeline.frames_out() >= last);
        last = timeline.frames_out();
        timeline.on_timeout(ts + 200 * MS);
        assert!(timeline.frames_out() >= last);
        last = timeline.frames_out();
    }
}

#[test]
fn encode_size_scales_1440p_down_to_1080p() {
    assert_eq!(encode_size((2560, 1440), Some(1080)), (1920, 1080));
}

#[test]
fn encode_size_rounds_odd_sides_down_to_even() {
    assert_eq!(encode_size((1921, 1081), None), (1920, 1080));
    assert_eq!(encode_size((1365, 767), Some(721)), (1282, 720));
}

#[test]
fn encode_size_keeps_native_without_a_limit() {
    assert_eq!(encode_size((2560, 1440), None), (2560, 1440));
}

#[test]
fn encode_size_keeps_native_when_limit_is_above_it() {
    assert_eq!(encode_size((1280, 720), Some(1080)), (1280, 720));
}

#[test]
fn encode_size_keeps_four_by_three_aspect() {
    assert_eq!(encode_size((1600, 1200), Some(1080)), (1440, 1080));
}

#[test]
fn bitrate_scales_down_to_a_smaller_encode_size() {
    let cfg = CaptureConfig {
        fps: 60,
        max_height: None,
        video_bitrate: 80_000_000,
        video_peak: 120_000_000,
        quality: Some(70),
        gop_frames: 60,
        audio: true,
        audio_bitrate: 192_000,
        cursor: false,
        bitrate_pixels: 3840 * 2160,
    };
    let scaled = cfg.for_encode_size(1920, 1080);
    assert_eq!(scaled.video_bitrate, 28_284_271);
    assert_eq!(scaled.video_peak, 42_426_407);
    assert_eq!(scaled.quality, Some(70));
    assert_eq!(cfg.for_encode_size(3840, 2160), cfg);
}
