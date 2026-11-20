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
