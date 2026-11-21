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
