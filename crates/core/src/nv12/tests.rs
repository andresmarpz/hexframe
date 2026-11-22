use super::*;

fn frame(width: usize, height: usize, luma: u8, u: u8, v: u8) -> Vec<u8> {
    let mut data = vec![luma; width * height];
    for _ in 0..width * height / 4 {
        data.extend([u, v]);
    }
    data
}

fn source(data: &[u8], width: usize, height: usize) -> Nv12Source<'_> {
    Nv12Source {
        data,
        pitch: width,
        rows: height,
        crop: (0, 0, width, height),
    }
}

#[test]
fn a_flat_picture_stays_flat_at_any_size() {
    let data = frame(64, 36, 90, 100, 150);
    let mut out = Vec::new();
    assert!(scale(&source(&data, 64, 36), 32, 18, &mut out));
    assert_eq!(out.len(), frame_len(32, 18));
    assert!(out[..32 * 18].iter().all(|&y| y == 90));
    assert!(out[32 * 18..].chunks(2).all(|uv| uv == [100, 150]));
    assert!(scale(&source(&data, 64, 36), 128, 72, &mut out));
    assert_eq!(out.len(), frame_len(128, 72));
    assert!(out[..128 * 72].iter().all(|&y| y == 90));
}

#[test]
fn the_same_size_is_a_copy() {
    let mut data = frame(16, 8, 0, 128, 128);
    for (i, y) in data[..16 * 8].iter_mut().enumerate() {
        *y = i as u8;
    }
    let mut out = Vec::new();
    assert!(scale(&source(&data, 16, 8), 16, 8, &mut out));
    assert_eq!(out, data);
}

#[test]
fn halving_averages_neighbours() {
    let mut data = frame(8, 4, 0, 128, 128);
    for row in 0..4 {
        for col in 0..8 {
            data[row * 8 + col] = if col % 2 == 0 { 40 } else { 80 };
        }
    }
    let mut out = Vec::new();
    assert!(scale(&source(&data, 8, 4), 4, 2, &mut out));
    assert!(out[..8].iter().all(|&y| y == 60));
}

#[test]
fn a_gradient_stays_in_order_when_scaled() {
    let mut data = frame(120, 40, 0, 128, 128);
    for row in 0..40 {
        for col in 0..120 {
            data[row * 120 + col] = (col * 2) as u8;
        }
    }
    let mut out = Vec::new();
    assert!(scale(&source(&data, 120, 40), 80, 26, &mut out));
    let first_row = &out[..80];
    assert!(first_row.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(first_row[0] <= 2);
    assert!(first_row[79] >= 236);
}

#[test]
fn the_crop_drops_padding_rows() {
    let width = 16;
    let coded = 12;
    let shown = 8;
    let mut data = frame(width, coded, 200, 128, 128);
    for row in shown..coded {
        data[row * width..(row + 1) * width].fill(0);
    }
    let mut out = Vec::new();
    let picture = Nv12Source {
        data: &data,
        pitch: width,
        rows: coded,
        crop: (0, 0, width, shown),
    };
    assert!(scale(&picture, 8, 4, &mut out));
    assert!(out[..8 * 4].iter().all(|&y| y == 200));
}

#[test]
fn a_cropped_corner_keeps_its_chroma() {
    let width = 16;
    let height = 8;
    let mut data = frame(width, height, 50, 100, 100);
    for row in 0..height {
        for col in 8..width {
            data[row * width + col] = 220;
        }
    }
    let chroma = width * height;
    for row in 0..height / 2 {
        for pair in 4..width / 2 {
            data[chroma + row * width + pair * 2] = 30;
            data[chroma + row * width + pair * 2 + 1] = 200;
        }
    }
    let mut out = Vec::new();
    let picture = Nv12Source {
        data: &data,
        pitch: width,
        rows: height,
        crop: (8, 0, 8, 8),
    };
    assert!(scale(&picture, 4, 4, &mut out));
    assert!(out[..16].iter().all(|&y| y == 220));
    assert!(out[16..].chunks(2).all(|uv| uv == [30, 200]));
}

#[test]
fn a_short_buffer_or_bad_crop_is_refused_without_panicking() {
    let data = frame(16, 8, 1, 2, 3);
    let mut out = Vec::new();
    assert!(!scale(&source(&data[..100], 16, 8), 8, 4, &mut out));
    let wide = Nv12Source {
        data: &data,
        pitch: 16,
        rows: 8,
        crop: (4, 0, 16, 8),
    };
    assert!(!scale(&wide, 8, 4, &mut out));
    assert!(!scale(&source(&data, 16, 8), 0, 0, &mut out));
}

mod property {
    use super::*;
    use proptest::prelude::*;

    fn noise(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect()
    }

    fn padded(tight: &[u8], width: usize, height: usize, pitch: usize, guard: u8) -> Vec<u8> {
        let mut data = vec![guard; pitch * height * 3 / 2];
        for row in 0..height {
            data[row * pitch..row * pitch + width]
                .copy_from_slice(&tight[row * width..(row + 1) * width]);
        }
        let chroma = pitch * height;
        let tight_chroma = width * height;
        for row in 0..height / 2 {
            let from = tight_chroma + row * width;
            data[chroma + row * pitch..chroma + row * pitch + width]
                .copy_from_slice(&tight[from..from + width]);
        }
        data
    }

    proptest! {
        #[test]
        fn scaling_never_panics_and_sizes_the_output_exactly(
            width in 1usize..80,
            height in 1usize..60,
            pad in 0usize..20,
            crop in (0usize..90, 0usize..70, 0usize..100, 0usize..80),
            target in (0usize..100, 0usize..80),
            seed in any::<u64>(),
        ) {
            let pitch = width + pad;
            let data = noise(seed, pitch * height * 3 / 2);
            let picture = Nv12Source { data: &data, pitch, rows: height, crop };
            let mut out = vec![7; 3];
            if scale(&picture, target.0, target.1, &mut out) {
                prop_assert_eq!(out.len(), frame_len(target.0 & !1, target.1 & !1));
            }
        }

        #[test]
        fn a_padded_pitch_scales_like_a_tight_one(
            half_width in 1usize..32,
            half_height in 1usize..24,
            pad in 1usize..40,
            target in (2usize..80, 2usize..60),
            seed in any::<u64>(),
            guard in any::<u8>(),
        ) {
            let (width, height) = (half_width * 2, half_height * 2);
            let tight = noise(seed, width * height * 3 / 2);
            let wide = padded(&tight, width, height, width + pad, guard);
            let mut expected = Vec::new();
            let mut actual = Vec::new();
            prop_assert!(scale(&source(&tight, width, height), target.0, target.1, &mut expected));
            let wide_picture = Nv12Source {
                data: &wide,
                pitch: width + pad,
                rows: height,
                crop: (0, 0, width, height),
            };
            prop_assert!(scale(&wide_picture, target.0, target.1, &mut actual));
            prop_assert_eq!(expected, actual);
        }

        #[test]
        fn flat_pictures_stay_flat(
            half_width in 1usize..32,
            half_height in 1usize..24,
            pad in 0usize..20,
            target in (2usize..96, 2usize..72),
            levels in (any::<u8>(), any::<u8>(), any::<u8>()),
        ) {
            let (width, height) = (half_width * 2, half_height * 2);
            let tight = frame(width, height, levels.0, levels.1, levels.2);
            let data = padded(&tight, width, height, width + pad, 0);
            let picture = Nv12Source {
                data: &data,
                pitch: width + pad,
                rows: height,
                crop: (0, 0, width, height),
            };
            let mut out = Vec::new();
            prop_assert!(scale(&picture, target.0, target.1, &mut out));
            let (w, h) = (target.0 & !1, target.1 & !1);
            prop_assert!(out[..w * h].iter().all(|&y| y == levels.0));
            prop_assert!(out[w * h..].chunks(2).all(|uv| uv == [levels.1, levels.2]));
        }

        #[test]
        fn the_last_chroma_row_is_written(
            half_width in 1usize..40,
            half_height in 1usize..40,
            target in (2usize..60, 2usize..60),
        ) {
            let (width, height) = (half_width * 2, half_height * 2);
            let data = frame(width, height, 100, 200, 60);
            let mut out = vec![0xAB; 5];
            prop_assert!(scale(&source(&data, width, height), target.0, target.1, &mut out));
            let (w, h) = (target.0 & !1, target.1 & !1);
            let last_row = &out[w * h + (h / 2 - 1) * w..];
            prop_assert_eq!(last_row.len(), w);
            prop_assert!(last_row.chunks(2).all(|uv| uv == [200, 60]));
        }

        #[test]
        fn short_buffers_are_refused(
            half_width in 1usize..20,
            half_height in 1usize..20,
            missing in 1usize..50,
        ) {
            let (width, height) = (half_width * 2, half_height * 2);
            let full = frame(width, height, 1, 2, 3);
            let cut = full.len().saturating_sub(missing);
            let mut out = Vec::new();
            let short = Nv12Source {
                data: &full[..cut],
                pitch: width,
                rows: height,
                crop: (0, 0, width, height),
            };
            prop_assert!(!scale(&short, width, height, &mut out));
        }
    }
}
