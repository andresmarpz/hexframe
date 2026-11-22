use super::*;
use crate::shape::{cover, crop_16_9};

fn flat(width: u32, height: u32, rgb: [u8; 3]) -> Frame {
    let rgba = (0..width * height)
        .flat_map(|_| [rgb[0], rgb[1], rgb[2], 255])
        .collect();
    Frame {
        width,
        height,
        rgba,
    }
}

#[test]
fn wide_frames_lose_their_sides() {
    assert_eq!(crop_16_9(2560, 1080), (320, 0, 1920, 1080));
}

#[test]
fn tall_frames_lose_top_and_bottom() {
    assert_eq!(crop_16_9(1280, 1024), (0, 152, 1280, 720));
}

#[test]
fn sixteen_nine_frames_are_kept_whole() {
    assert_eq!(crop_16_9(1920, 1080), (0, 0, 1920, 1080));
}

#[test]
fn big_frames_shrink_to_the_thumbnail_size() {
    let shaped = cover(&flat(1920, 1080, [10, 20, 30])).unwrap();
    assert_eq!(
        (shaped.width(), shaped.height()),
        (THUMB_WIDTH, THUMB_HEIGHT)
    );
}

#[test]
fn a_mismatched_frame_is_an_error() {
    let frame = Frame {
        width: 10,
        height: 10,
        rgba: vec![0; 12],
    };
    assert!(matches!(cover(&frame), Err(ThumbError::Decode(_))));
}

#[test]
fn written_thumbnails_are_jpeg_files_without_leftovers() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("clips").join("a.jpg");
    write_jpeg(&flat(800, 450, [200, 40, 40]), &dest).unwrap();
    let bytes = std::fs::read(&dest).unwrap();
    assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF]);
    let decoded = image::load_from_memory(&bytes).unwrap();
    assert_eq!(
        (decoded.width(), decoded.height()),
        (THUMB_WIDTH, THUMB_HEIGHT)
    );
    let names: Vec<_> = std::fs::read_dir(dest.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1);
}

#[test]
fn a_missing_video_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("t.jpg");
    assert!(write_thumbnail(&dir.path().join("nope.mp4"), 1.0, &dest).is_err());
    assert!(!dest.exists());
}

fn nv12_plane(width: usize, height: usize, y: u8, u: u8, v: u8) -> Vec<u8> {
    let mut data = vec![y; width * height];
    data.extend((0..width * height / 4).flat_map(|_| [u, v]));
    data
}

#[test]
fn limited_range_black_and_white_map_to_the_ends() {
    let black =
        crate::nv12::nv12_to_rgba(&nv12_plane(4, 4, 16, 128, 128), 4, 4, (0, 0, 4, 4), false);
    let white =
        crate::nv12::nv12_to_rgba(&nv12_plane(4, 4, 235, 128, 128), 4, 4, (0, 0, 4, 4), false);
    assert_eq!(&black.rgba[..4], &[0, 0, 0, 255]);
    assert_eq!(&white.rgba[..4], &[255, 255, 255, 255]);
}

#[test]
fn red_chroma_gives_a_red_pixel() {
    let red = crate::nv12::nv12_to_rgba(&nv12_plane(4, 4, 63, 102, 240), 4, 4, (0, 0, 4, 4), false);
    let [r, g, b, _] = [red.rgba[0], red.rgba[1], red.rgba[2], red.rgba[3]];
    assert!(r > 220 && g < 40 && b < 40, "{r} {g} {b}");
}

#[test]
fn crop_selects_the_requested_window() {
    let frame =
        crate::nv12::nv12_to_rgba(&nv12_plane(8, 8, 235, 128, 128), 8, 8, (2, 2, 4, 4), false);
    assert_eq!((frame.width, frame.height, frame.rgba.len()), (4, 4, 64));
}

mod storyboard {
    use std::path::PathBuf;

    use super::flat;
    use crate::sheet::Packer;
    use crate::{Pass, SheetLayout, write_sheets};

    const LAYOUT: SheetLayout = SheetLayout {
        tile_width: 16,
        tile_height: 9,
        columns: 4,
        rows: 2,
    };

    fn sheet_in(dir: &std::path::Path) -> impl Fn(usize) -> PathBuf + '_ {
        move |index| dir.join(format!("sheet-{index}.jpg"))
    }

    fn pixel(sheet: &std::path::Path, x: u32, y: u32) -> [u8; 3] {
        let image = image::open(sheet).unwrap().to_rgb8();
        image.get_pixel(x, y).0
    }

    #[test]
    fn tiles_fill_a_sheet_row_by_row_and_a_full_sheet_is_written_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let dest = sheet_in(dir.path());
        let mut packer = Packer::new(LAYOUT);
        for index in 0..8u8 {
            packer
                .push(Some(&flat(160, 90, [index * 30, 0, 0])), &dest)
                .unwrap();
        }
        assert_eq!(packer.written, 1);
        let sheet = dest(0);
        assert_eq!(image::image_dimensions(&sheet).unwrap(), (64, 18));
        assert!(pixel(&sheet, 2, 2)[0] < 12);
        let second = pixel(&sheet, 18, 2)[0];
        assert!((i32::from(second) - 30).abs() < 12, "{second}");
        let fifth = pixel(&sheet, 2, 11)[0];
        assert!((i32::from(fifth) - 120).abs() < 12, "{fifth}");
    }

    #[test]
    fn a_partial_last_sheet_is_written_on_flush() {
        let dir = tempfile::tempdir().unwrap();
        let dest = sheet_in(dir.path());
        let mut packer = Packer::new(LAYOUT);
        for _ in 0..9 {
            packer.push(Some(&flat(64, 36, [0, 90, 0])), &dest).unwrap();
        }
        packer.flush(&dest).unwrap();
        assert_eq!(packer.written, 2);
        assert!(dest(1).is_file());
        packer.flush(&dest).unwrap();
        assert_eq!(packer.written, 2);
    }

    #[test]
    fn a_failed_frame_repeats_the_tile_before_it() {
        let dir = tempfile::tempdir().unwrap();
        let dest = sheet_in(dir.path());
        let mut packer = Packer::new(LAYOUT);
        packer
            .push(Some(&flat(160, 90, [0, 0, 200])), &dest)
            .unwrap();
        packer.push(None, &dest).unwrap();
        packer.flush(&dest).unwrap();
        let blue = pixel(&dest(0), 18, 2);
        assert!(blue[2] > 170, "{blue:?}");
    }

    #[test]
    fn a_missing_video_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let result = write_sheets(
            &dir.path().join("nope.mp4"),
            &[0.0, 1.0],
            LAYOUT,
            &sheet_in(dir.path()),
            &|| false,
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[cfg(any(target_os = "macos", windows))]
    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../app/fixtures/review.mp4")
    }

    #[cfg(any(target_os = "macos", windows))]
    #[test]
    fn a_real_video_gives_one_distinct_tile_per_time() {
        let dir = tempfile::tempdir().unwrap();
        let layout = SheetLayout {
            tile_width: 160,
            tile_height: 90,
            columns: 4,
            rows: 3,
        };
        let times: Vec<f64> = (0..10).map(f64::from).collect();
        let pass = write_sheets(&fixture(), &times, layout, &sheet_in(dir.path()), &|| false)
            .expect("storyboard");
        assert_eq!(pass, Pass::Finished { sheets: 1 });
        let sheet = image::open(dir.path().join("sheet-0.jpg"))
            .unwrap()
            .to_rgb8();
        assert_eq!(sheet.dimensions(), (640, 270));
        let tile = |slot: u32| -> Vec<u8> {
            let (x, y) = (slot % 4 * 160, slot / 4 * 90);
            image::imageops::crop_imm(&sheet, x, y, 160, 90)
                .to_image()
                .into_raw()
        };
        assert!(
            tile(0).iter().any(|v| *v > 20),
            "the first tile is not black"
        );
        assert_ne!(tile(0), tile(8), "tiles at different times differ");
        assert!(
            sheet.get_pixel(560, 225).0.iter().all(|v| *v < 8),
            "unused slots stay black"
        );
    }

    #[cfg(any(target_os = "macos", windows))]
    #[test]
    fn a_stop_request_ends_the_pass_without_writing_a_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let times: Vec<f64> = (0..10).map(f64::from).collect();
        let pass = write_sheets(&fixture(), &times, LAYOUT, &sheet_in(dir.path()), &|| true)
            .expect("pass");
        assert_eq!(pass, Pass::Stopped);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

fn gradient_nv12(width: usize, height: usize, pitch: usize, guard: u8) -> Vec<u8> {
    let mut data = vec![guard; pitch * height * 3 / 2];
    for row in 0..height {
        for col in 0..width {
            data[row * pitch + col] = 16 + ((row * 7 + col * 3) % 200) as u8;
        }
    }
    let chroma = pitch * height;
    for row in 0..height / 2 {
        for pair in 0..width / 2 {
            data[chroma + row * pitch + pair * 2] = 90 + ((row + pair) % 60) as u8;
            data[chroma + row * pitch + pair * 2 + 1] = 120 + ((row * 3 + pair) % 80) as u8;
        }
    }
    data
}

#[test]
fn a_padded_pitch_converts_like_a_tight_buffer() {
    for (width, height) in [(16, 8), (18, 10), (33, 17), (31, 9), (2, 2)] {
        let tight = gradient_nv12(width, height, width, 0);
        let padded = gradient_nv12(width, height, width + 17, 0xEE);
        let window = (0, 0, width & !1, height & !1);
        let expected = crate::nv12::nv12_to_rgba(&tight, width, height, window, false);
        let actual = crate::nv12::nv12_to_rgba(&padded, width + 17, height, window, false);
        assert_eq!(actual.rgba, expected.rgba, "{width}x{height}");
        assert_eq!(
            (actual.width, actual.height),
            (expected.width, expected.height)
        );
    }
}

#[test]
fn odd_sizes_use_the_chroma_of_the_last_row_and_column() {
    let (width, height) = (5, 5);
    let pitch = width + 17;
    let mut data = vec![0xEE; pitch * 8];
    for row in 0..height {
        data[row * pitch..row * pitch + width].fill(126);
    }
    let chroma = pitch * height;
    for row in 0..3 {
        for pair in 0..3 {
            data[chroma + row * pitch + pair * 2] = 128;
            data[chroma + row * pitch + pair * 2 + 1] = 128;
        }
    }
    data[chroma + 2 * pitch + 4] = 102;
    data[chroma + 2 * pitch + 5] = 240;
    let frame = crate::nv12::nv12_to_rgba(&data, pitch, height, (0, 0, width, height), true);
    let pixel = |col: usize, row: usize| {
        let at = (row * width + col) * 4;
        [frame.rgba[at], frame.rgba[at + 1], frame.rgba[at + 2]]
    };
    assert_eq!(pixel(0, 0), [126, 126, 126]);
    assert!(pixel(4, 4)[0] > 200);
    assert!(pixel(4, 4)[2] < 126);
    assert_eq!(pixel(3, 4), pixel(2, 4));
}
