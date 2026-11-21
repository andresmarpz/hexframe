use super::*;

fn bounds(left: i32, top: i32, width: i32, height: i32) -> ScreenRect {
    ScreenRect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

fn crop(left: u32, top: u32, width: u32, height: u32) -> CropRect {
    CropRect {
        left,
        top,
        width,
        height,
    }
}

#[test]
fn borderless_uses_the_full_frame() {
    let result = client_crop((1920, 1080), bounds(0, 0, 1920, 1080), (0, 0), (1920, 1080));
    assert_eq!(result, CropRect::full((1920, 1080)));
    assert!(result.is_full((1920, 1080)));
}

#[test]
fn borderless_with_odd_frame_stays_exact() {
    let result = client_crop((1921, 1081), bounds(0, 0, 1921, 1081), (0, 0), (1921, 1081));
    assert_eq!(result, CropRect::full((1921, 1081)));
}

#[test]
fn windowed_at_100_percent_drops_borders_and_caption() {
    let result = client_crop(
        (1616, 939),
        bounds(200, 100, 1616, 939),
        (208, 131),
        (1600, 900),
    );
    assert_eq!(result, crop(8, 31, 1600, 900));
}

#[test]
fn windowed_at_150_percent_scales_offset_and_size() {
    let result = client_crop(
        (1620, 960),
        bounds(100, 50, 1080, 640),
        (108, 82),
        (1064, 600),
    );
    assert_eq!(result, crop(12, 48, 1596, 900));
}

#[test]
fn windowed_on_a_second_monitor_uses_relative_offsets() {
    let result = client_crop(
        (1616, 939),
        bounds(-1800, 40, 1616, 939),
        (-1792, 71),
        (1600, 900),
    );
    assert_eq!(result, crop(8, 31, 1600, 900));
}

#[test]
fn odd_sizes_round_down_to_even() {
    let result = client_crop((1617, 940), bounds(0, 0, 1617, 940), (8, 31), (1601, 901));
    assert_eq!(result, crop(8, 31, 1600, 900));
}

#[test]
fn zero_client_size_means_minimized_and_uses_the_full_frame() {
    let result = client_crop((1616, 939), bounds(0, 0, 1616, 939), (8, 31), (0, 0));
    assert_eq!(result, CropRect::full((1616, 939)));
}

#[test]
fn client_outside_the_frame_uses_the_full_frame() {
    let beyond = client_crop((1616, 939), bounds(0, 0, 1616, 939), (8, 31), (2000, 900));
    assert_eq!(beyond, CropRect::full((1616, 939)));
    let before = client_crop(
        (1616, 939),
        bounds(0, 0, 1616, 939),
        (-300, 31),
        (1600, 900),
    );
    assert_eq!(before, CropRect::full((1616, 939)));
}

#[test]
fn empty_frame_bounds_use_the_full_frame() {
    let result = client_crop((1616, 939), bounds(0, 0, 0, 0), (8, 31), (1600, 900));
    assert_eq!(result, CropRect::full((1616, 939)));
}

#[test]
fn one_pixel_rounding_overshoot_is_clamped() {
    let result = client_crop((1617, 940), bounds(0, 0, 1616, 939), (8, 31), (1609, 908));
    assert_eq!(result.left, 8);
    assert!(result.left + result.width <= 1617);
    assert!(result.top + result.height <= 940);
}
