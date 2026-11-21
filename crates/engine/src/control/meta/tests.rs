use super::*;

#[test]
fn file_secs_subtracts_the_media_offset() {
    let at = 12 * HNS_PER_SEC;
    assert!((file_secs(at, 2 * HNS_PER_SEC, 0.0) - 10.0).abs() < 1e-9);
    assert!((file_secs(at, 2 * HNS_PER_SEC, 1.5) - 8.5).abs() < 1e-9);
}

#[test]
fn video_offset_comes_from_the_dts_in_seconds() {
    let packet = Packet {
        track: Track::Video,
        dts: 3 * i64::from(VideoFormat::TIMESCALE) / 2,
        pts: 0,
        duration: 1500,
        key: true,
        data: Arc::from(vec![0u8; 1]),
    };
    assert!((video_dts_secs(&packet) - 1.5).abs() < 1e-9);
}
