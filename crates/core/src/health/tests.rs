use std::sync::Arc;

use super::*;
use crate::media::{Packet, Track, VideoFormat};

fn format() -> VideoFormat {
    VideoFormat {
        width: 1920,
        height: 1080,
        fps: 60,
        sps: Vec::new(),
        pps: Vec::new(),
    }
}

fn video(tick: i64) -> Packet {
    Packet {
        track: Track::Video,
        dts: tick * 1500,
        pts: tick * 1500,
        duration: 1500,
        key: tick % 60 == 0,
        data: Arc::from(vec![0u8; 1000]),
    }
}

fn tracker() -> HealthTracker {
    HealthTracker::new(0, &format(), "Test encoder (quality 70)", Some(48_000))
}

#[test]
fn a_write_failure_marks_the_video_stopped_once() {
    let mut tracker = tracker();
    for tick in 0..120 {
        tracker.on_packet(&video(tick));
    }
    tracker.on_write_failure();
    for tick in 120..240 {
        tracker.on_packet(&video(tick));
    }
    tracker.on_write_failure();
    let health = tracker.finish(None);
    assert_eq!(health.video_stopped_secs, Some(2.0));
    assert!(
        health
            .findings()
            .iter()
            .any(|f| f.area == Area::File && f.level == Level::Bad)
    );
}
