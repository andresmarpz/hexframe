use super::*;

fn labels(options: &[ResolutionOption]) -> Vec<&str> {
    options.iter().map(|o| o.label.as_str()).collect()
}

fn heights(options: &[ResolutionOption]) -> Vec<Option<u32>> {
    options.iter().map(|o| o.resolution.height()).collect()
}

#[test]
fn a_4k_display_offers_the_whole_ladder_below_it() {
    let options = resolution_options(&[(3840, 2160)], Resolution::Height(1080));
    assert_eq!(
        labels(&options),
        [
            "Native (3840 × 2160)",
            "2560 × 1440",
            "2134 × 1200",
            "1920 × 1080",
            "1600 × 900",
            "1280 × 720",
        ]
    );
    assert_eq!(
        heights(&options),
        [
            None,
            Some(1440),
            Some(1200),
            Some(1080),
            Some(900),
            Some(720)
        ]
    );
}

#[test]
fn a_1440p_display_starts_below_its_own_height() {
    let options = resolution_options(&[(2560, 1440)], Resolution::Height(1080));
    assert_eq!(
        labels(&options),
        [
            "Native (2560 × 1440)",
            "2134 × 1200",
            "1920 × 1080",
            "1600 × 900",
            "1280 × 720",
        ]
    );
}

#[test]
fn a_16_by_10_display_scales_the_width_by_its_own_ratio() {
    let options = resolution_options(&[(1920, 1200)], Resolution::Height(1080));
    assert_eq!(
        labels(&options),
        [
            "Native (1920 × 1200)",
            "1728 × 1080",
            "1440 × 900",
            "1152 × 720"
        ]
    );
}

#[test]
fn a_1080p_display_offers_only_lower_heights() {
    let options = resolution_options(&[(1920, 1080)], Resolution::Height(900));
    assert_eq!(
        labels(&options),
        ["Native (1920 × 1080)", "1600 × 900", "1280 × 720"]
    );
}

#[test]
fn no_displays_shows_native_and_the_whole_ladder() {
    let options = resolution_options(&[], Resolution::Native);
    assert_eq!(
        labels(&options),
        ["Native", "2160p", "1440p", "1200p", "1080p", "900p", "720p"]
    );
}

#[test]
fn the_tallest_display_decides() {
    let options = resolution_options(&[(1920, 1080), (3840, 2160)], Resolution::Native);
    assert_eq!(options[0].label, "Native (3840 × 2160)");
    let options = resolution_options(&[(1920, 1080), (2560, 1440)], Resolution::Native);
    assert_eq!(options[0].label, "Native (2560 × 1440)");
}

#[test]
fn a_saved_height_outside_the_list_is_kept_in_order() {
    let options = resolution_options(&[(1920, 1080)], Resolution::Height(1440));
    assert_eq!(
        labels(&options),
        [
            "Native (1920 × 1080)",
            "2560 × 1440",
            "1600 × 900",
            "1280 × 720"
        ]
    );
    let options = resolution_options(&[(1920, 1080)], Resolution::Height(1080));
    assert_eq!(heights(&options)[1], Some(1080));
    let options = resolution_options(&[], Resolution::Height(800));
    assert!(heights(&options).contains(&Some(800)));
    let h = heights(&options);
    assert!(h[1..].windows(2).all(|w| w[0] > w[1]));
}

#[test]
fn a_saved_height_in_the_list_is_not_repeated() {
    let options = resolution_options(&[(3840, 2160)], Resolution::Height(1440));
    assert_eq!(
        heights(&options)
            .iter()
            .filter(|h| **h == Some(1440))
            .count(),
        1
    );
}

#[test]
fn zero_sized_displays_count_as_no_display() {
    let options = resolution_options(&[(0, 0)], Resolution::Native);
    assert_eq!(options[0].label, "Native");
}

#[test]
fn old_settings_values_load_to_the_same_height() {
    for (old, height) in [("P1440", 1440), ("P1080", 1080), ("P720", 720)] {
        let json = format!("\"{old}\"");
        assert_eq!(
            serde_json::from_str::<Resolution>(&json).unwrap(),
            Resolution::Height(height)
        );
    }
    assert_eq!(
        serde_json::from_str::<Resolution>("\"Native\"").unwrap(),
        Resolution::Native
    );
}

#[test]
fn new_values_round_trip() {
    for value in [Resolution::Native, Resolution::Height(1200)] {
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<Resolution>(&json).unwrap(), value);
    }
}

#[test]
fn an_old_settings_file_keeps_its_resolution() {
    let settings: crate::settings::Settings =
        serde_json::from_str(r#"{"resolution":"P1440","fps":30}"#).unwrap();
    assert_eq!(settings.resolution, Resolution::Height(1440));
    assert_eq!(settings.fps, 30);
}
