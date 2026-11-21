use super::*;

const HD60: VideoInfo = VideoInfo {
    width: 1920,
    height: 1080,
    fps: 60,
};

fn range(start: f64, end: f64) -> ExportRange {
    ExportRange::new(start, end)
}

fn expected_bytes(plan: EncodePlan, secs: f64) -> f64 {
    f64::from(plan.video_bps + plan.audio_bps) * secs / 8.0
}

#[test]
fn a_short_discord_clip_keeps_720p60_and_fits_the_budget() {
    let plan = plan_discord(10.0, HD60).unwrap();
    assert_eq!((plan.width, plan.height, plan.fps), (1280, 720, 60));
    assert_eq!(plan.audio_bps, AUDIO_BITRATE);
    assert!(expected_bytes(plan, 10.0) <= DISCORD_LIMIT_BYTES as f64 * 0.9 + 1.0);
    assert!(expected_bytes(plan, 10.0) > DISCORD_LIMIT_BYTES as f64 * 0.89);
}

#[test]
fn a_longer_clip_drops_to_30_fps_and_less_bitrate() {
    let short = plan_discord(10.0, HD60).unwrap();
    let long = plan_discord(60.0, HD60).unwrap();
    assert_eq!(long.fps, 30);
    assert_eq!(long.height, 720);
    assert!(long.video_bps < short.video_bps);
    assert!(expected_bytes(long, 60.0) <= DISCORD_LIMIT_BYTES as f64 * 0.9 + 1.0);
}

#[test]
fn a_starved_clip_steps_down_the_resolution() {
    let plan = plan_discord(150.0, HD60).unwrap();
    assert_eq!((plan.width, plan.height), (852, 480));
    let plan = plan_discord(250.0, HD60).unwrap();
    assert_eq!((plan.width, plan.height), (640, 360));
}

#[test]
fn a_clip_that_cannot_fit_is_refused_with_the_longest_length() {
    let max = max_discord_secs();
    assert!((250..=270).contains(&max));
    assert!(plan_discord(f64::from(max), HD60).is_ok());
    assert_eq!(
        plan_discord(f64::from(max) + 1.0, HD60),
        Err(PlanError::TooLong { max_secs: max })
    );
}

#[test]
fn the_plan_never_upscales_and_keeps_even_sizes() {
    let small = VideoInfo {
        width: 854,
        height: 480,
        fps: 30,
    };
    let plan = plan_discord(10.0, small).unwrap();
    assert_eq!((plan.width, plan.height, plan.fps), (854, 480, 30));
    let odd = VideoInfo {
        width: 1366,
        height: 768,
        fps: 60,
    };
    let plan = plan_discord(20.0, odd).unwrap();
    assert_eq!(plan.height, 720);
    assert_eq!(plan.width % 2, 0);
    assert_eq!(plan.width, 1280);
    let tall = VideoInfo {
        width: 1000,
        height: 1001,
        fps: 24,
    };
    let plan = plan_discord(20.0, tall).unwrap();
    assert_eq!((plan.height % 2, plan.width % 2, plan.fps), (0, 0, 24));
}

#[test]
fn a_zero_length_or_unknown_source_still_plans() {
    assert!(plan_discord(0.0, HD60).is_ok());
    let unknown = VideoInfo {
        width: 0,
        height: 0,
        fps: 0,
    };
    let plan = plan_discord(10.0, unknown).unwrap();
    assert_eq!((plan.fps, plan.width % 2, plan.height % 2), (30, 0, 0));
}

#[test]
fn the_retry_lowers_the_bitrate_so_an_oversized_file_would_fit() {
    let first = plan_discord(30.0, HD60).unwrap();
    let measured = 11_200_000u64;
    let second = retry_plan(first, measured, 30.0, HD60).unwrap();
    assert!(second.video_bps < first.video_bps);
    let video_bits = f64::from(first.video_bps) * 30.0;
    let audio_bits = f64::from(first.audio_bps) * 30.0;
    let overhead = measured as f64 * 8.0 - video_bits - audio_bits;
    let predicted = (f64::from(second.video_bps) * 30.0 + audio_bits + overhead) / 8.0;
    assert!(predicted < DISCORD_LIMIT_BYTES as f64);
}

#[test]
fn the_retry_always_lowers_even_when_the_file_was_nearly_small_enough() {
    let first = plan_discord(30.0, HD60).unwrap();
    let second = retry_plan(first, DISCORD_LIMIT_BYTES + 1, 30.0, HD60).unwrap();
    assert!(f64::from(second.video_bps) <= f64::from(first.video_bps) * 0.95);
}

#[test]
fn a_hopeless_retry_is_refused() {
    let first = plan_discord(250.0, HD60).unwrap();
    assert!(matches!(
        retry_plan(first, 40_000_000, 250.0, HD60),
        Err(PlanError::TooLong { .. })
    ));
}

#[test]
fn a_moment_is_fifteen_seconds_each_side() {
    assert_eq!(ExportRange::moment(100.0, 1800.0), range(85.0, 115.0));
}

#[test]
fn a_moment_near_the_edges_keeps_its_length() {
    assert_eq!(ExportRange::moment(4.0, 1800.0), range(0.0, 30.0));
    assert_eq!(ExportRange::moment(1795.0, 1800.0), range(1770.0, 1800.0));
    assert_eq!(ExportRange::moment(5.0, 12.0), range(0.0, 12.0));
}

#[test]
fn the_bounds_pad_the_range_and_stay_inside_the_game() {
    assert_eq!(range(100.0, 120.0).bounds(1800.0), (70.0, 150.0));
    assert_eq!(range(10.0, 20.0).bounds(1800.0), (0.0, 50.0));
    assert_eq!(range(1790.0, 1800.0).bounds(1800.0), (1760.0, 1800.0));
}

#[test]
fn handles_stay_in_bounds_and_a_second_apart() {
    let r = range(100.0, 120.0);
    let bounds = r.bounds(1800.0);
    assert_eq!(r.with_start(10.0, bounds).start, 70.0);
    assert_eq!(r.with_start(119.5, bounds).start, 119.0);
    assert_eq!(r.with_end(1000.0, bounds).end, 150.0);
    assert_eq!(r.with_end(100.2, bounds).end, 101.0);
    assert_eq!(r.with_start(105.0, bounds), range(105.0, 120.0));
}

#[test]
fn a_reversed_range_is_put_in_order() {
    assert_eq!(ExportRange::new(9.0, 3.0), range(3.0, 9.0));
    assert_eq!(range(3.0, 9.5).duration(), 6.5);
}

#[test]
fn file_names_carry_champion_times_and_preset() {
    assert_eq!(
        file_stem("Ahri", range(754.2, 778.0), ExportPreset::Original),
        "Ahri_12-34_12-58_Original"
    );
    assert_eq!(
        file_stem("Kai'Sa", range(5.0, 35.0), ExportPreset::Discord),
        "KaiSa_00-05_00-35_Discord"
    );
    assert_eq!(
        file_stem("../..", range(0.0, 1.0), ExportPreset::Discord),
        "Game_00-00_00-01_Discord"
    );
}

#[test]
fn the_presets_read_plainly() {
    assert_eq!(ExportPreset::Original.label(), "Original quality");
    assert_eq!(ExportPreset::Discord.label(), "For Discord (under 10 MB)");
    assert!(ExportPreset::Original.note().contains("keyframe"));
}
