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

#[test]
fn fit_crop_keeps_a_rect_inside_the_frame() {
    let inside = crop(8, 31, 1600, 900);
    assert_eq!(fit_crop(inside, (1616, 939)), inside);
    let edge = crop(16, 39, 1600, 900);
    assert_eq!(fit_crop(edge, (1616, 939)), edge);
}

#[test]
fn fit_crop_returns_the_full_frame_for_bad_rects() {
    let frame = (1616, 939);
    let full = CropRect::full(frame);
    assert_eq!(fit_crop(crop(0, 0, 0, 100), frame), full);
    assert_eq!(fit_crop(crop(0, 0, 100, 0), frame), full);
    assert_eq!(fit_crop(crop(17, 0, 1600, 900), frame), full);
    assert_eq!(fit_crop(crop(0, 40, 1600, 900), frame), full);
    assert_eq!(fit_crop(crop(u32::MAX, 0, 10, 10), frame), full);
    assert_eq!(fit_crop(crop(0, 0, u32::MAX, u32::MAX), frame), full);
    assert_eq!(fit_crop(crop(2000, 2000, 10, 10), frame), full);
}

mod property {
    use super::*;
    use proptest::prelude::*;

    fn inside_frame(rect: CropRect, frame: (u32, u32)) -> bool {
        rect.width > 0
            && rect.height > 0
            && u64::from(rect.left) + u64::from(rect.width) <= u64::from(frame.0)
            && u64::from(rect.top) + u64::from(rect.height) <= u64::from(frame.1)
    }

    proptest! {
        #[test]
        fn fit_crop_always_lands_inside_the_frame(
            frame in (1u32..8000, 1u32..8000),
            left in any::<u32>(),
            top in any::<u32>(),
            width in any::<u32>(),
            height in any::<u32>(),
        ) {
            let fitted = fit_crop(crop(left, top, width, height), frame);
            prop_assert!(inside_frame(fitted, frame));
        }

        #[test]
        fn client_crop_stays_inside_the_frame(
            frame in (0u32..6000, 0u32..4000),
            monitor in (-8000i32..8000, -8000i32..8000),
            monitor_size in (-100i32..6000, -100i32..4000),
            client_origin in (-8000i32..8000, -8000i32..8000),
            client_size in (-100i32..6000, -100i32..4000),
        ) {
            let result = client_crop(
                frame,
                bounds(monitor.0, monitor.1, monitor_size.0, monitor_size.1),
                client_origin,
                client_size,
            );
            if frame.0 > 0 && frame.1 > 0 {
                prop_assert!(inside_frame(result, frame));
            } else {
                prop_assert_eq!(result, CropRect::full(frame));
            }
        }

        #[test]
        fn client_crop_ignores_the_monitor_origin(
            frame in (2u32..4000, 2u32..3000),
            dpi in prop::sample::select(vec![100i32, 125, 150, 200]),
            monitor in (-5000i32..5000, -5000i32..5000),
            shift in (-5000i32..5000, -5000i32..5000),
            inset in (0i32..400, 0i32..300),
            client_size in (-50i32..3000, -50i32..2000),
        ) {
            let width = (frame.0 as i32 * 100 / dpi).max(1);
            let height = (frame.1 as i32 * 100 / dpi).max(1);
            let at = |origin: (i32, i32)| {
                client_crop(
                    frame,
                    bounds(origin.0, origin.1, width, height),
                    (origin.0 + inset.0, origin.1 + inset.1),
                    client_size,
                )
            };
            let moved = (monitor.0 + shift.0, monitor.1 + shift.1);
            prop_assert_eq!(at(monitor), at(moved));
            prop_assert!(inside_frame(at(monitor), frame));
        }
    }
}
