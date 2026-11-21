use super::*;

fn config(video_bitrate: u32, audio: bool) -> CaptureConfig {
    CaptureConfig {
        audio,
        audio_bitrate: 192_000,
        video_bitrate,
        ..crate::settings::Settings::default().capture_config(1080)
    }
}

#[test]
fn the_bitrate_is_bytes_per_second_with_the_audio_when_it_is_on() {
    assert_eq!(bytes_per_sec(&config(8_000_000, false)), 1_000_000);
    assert_eq!(bytes_per_sec(&config(8_000_000, true)), 1_024_000);
}

#[test]
fn a_slow_recording_needs_the_floor() {
    assert_eq!(start_threshold(1_500_000), START_FLOOR_BYTES);
    assert_eq!(start_threshold(0), START_FLOOR_BYTES);
}

#[test]
fn a_fast_recording_needs_ten_minutes_of_itself() {
    let rate = 4 * BYTES_PER_MB;
    assert_eq!(start_threshold(rate), rate * 600);
    assert!(start_threshold(rate) > START_FLOOR_BYTES);
}

#[test]
fn a_huge_bitrate_does_not_overflow() {
    assert_eq!(start_threshold(u64::MAX), u64::MAX);
}

#[test]
fn starting_needs_the_threshold_exactly() {
    let rate = 1_500_000;
    assert!(can_start(START_FLOOR_BYTES, rate));
    assert!(!can_start(START_FLOOR_BYTES - 1, rate));
    assert!(!can_start(0, rate));
}

#[test]
fn a_running_recording_stops_below_the_hard_limit_only() {
    assert!(!must_stop(STOP_BYTES));
    assert!(must_stop(STOP_BYTES - 1));
    assert!(must_stop(0));
}

#[test]
fn free_space_reads_in_gigabytes_with_one_decimal() {
    assert_eq!(gb_text(1_450_000_000), "1.4 GB");
    assert_eq!(gb_text(1_460_000_000), "1.5 GB");
    assert_eq!(gb_text(0), "0.0 GB");
    assert_eq!(gb_text(120 * BYTES_PER_GB), "120.0 GB");
}
