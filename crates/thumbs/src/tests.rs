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
