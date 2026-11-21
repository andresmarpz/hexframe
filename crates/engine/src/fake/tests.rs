use std::time::{Duration, Instant};

use crossbeam_channel::bounded;

use super::*;

fn config() -> CaptureConfig {
    CaptureConfig {
        fps: 60,
        max_height: None,
        video_bitrate: 8_000_000,
        video_peak: 12_000_000,
        gop_frames: 60,
        audio: true,
        audio_bitrate: 192_000,
        bitrate_pixels: 0,
    }
}

fn collect_fast(seconds: i64) -> (Vec<MediaEvent>, Vec<Packet>) {
    let (tx, rx) = bounded(256);
    let session = FakeMediaSource::fast()
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    let limit = seconds * i64::from(VideoFormat::TIMESCALE);
    let mut events = Vec::new();
    let mut packets = Vec::new();
    for event in rx.iter() {
        if let MediaEvent::Packet(p) = &event {
            let scale = match p.track {
                Track::Video => i64::from(VideoFormat::TIMESCALE),
                Track::Audio => i64::from(SAMPLE_RATE),
            };
            if p.dts * i64::from(VideoFormat::TIMESCALE) / scale >= limit {
                break;
            }
            packets.push(p.clone());
        }
        events.push(event);
    }
    session.stop();
    (events, packets)
}

fn of(packets: &[Packet], track: Track) -> Vec<&Packet> {
    packets.iter().filter(|p| p.track == track).collect()
}

#[test]
fn started_comes_first_with_formats() {
    let (events, _) = collect_fast(1);
    let MediaEvent::Started {
        video,
        audio,
        encoder,
        ..
    } = &events[0]
    else {
        panic!("first event is not Started");
    };
    assert_eq!((video.width, video.height, video.fps), (640, 360, 60));
    let audio = audio.as_ref().unwrap();
    assert_eq!((audio.sample_rate, audio.channels), (48_000, 2));
    assert_eq!(audio.asc, [0x11, 0x90]);
    assert_eq!(encoder, "fake");
}

#[test]
fn dts_is_continuous_per_track_across_loops() {
    let (_, packets) = collect_fast(10);
    let video = of(&packets, Track::Video);
    assert!(video.len() >= 590);
    for (i, p) in video.iter().enumerate() {
        assert_eq!(p.dts, i as i64 * 1500);
        assert_eq!(p.pts, p.dts);
        assert_eq!(p.duration, 1500);
    }
    let audio = of(&packets, Track::Audio);
    assert!(audio.len() > 400);
    for (i, p) in audio.iter().enumerate() {
        assert_eq!(p.dts, i as i64 * 1024);
        assert_eq!(p.duration, 1024);
    }
}

#[test]
fn video_keys_fall_every_60_frames() {
    let (_, packets) = collect_fast(10);
    for (i, p) in of(&packets, Track::Video).iter().enumerate() {
        assert_eq!(p.key, i % 60 == 0, "frame {i}");
    }
}

#[test]
fn video_data_is_avcc_and_audio_has_no_adts_header() {
    let (_, packets) = collect_fast(1);
    let video = of(&packets, Track::Video)[0];
    let len = u32::from_be_bytes(video.data[..4].try_into().unwrap()) as usize;
    assert!(len + 4 <= video.data.len());
    assert_eq!(nal_type(video.data[4]), 5);
    let audio = of(&packets, Track::Audio)[0];
    assert!(!(audio.data[0] == 0xFF && audio.data[1] & 0xF0 == 0xF0));
}

#[test]
fn realtime_paces_packets_and_stop_is_prompt() {
    let (tx, rx) = bounded(256);
    let session = FakeMediaSource::new()
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    let begin = Instant::now();
    let mut last_video = 0;
    while begin.elapsed() < Duration::from_millis(500) {
        if let Ok(MediaEvent::Packet(p)) = rx.recv_timeout(Duration::from_millis(50))
            && p.track == Track::Video
        {
            last_video = p.dts;
        }
    }
    let frames = last_video / 1500;
    assert!((20..=40).contains(&frames), "frames in 0.5 s: {frames}");
    let stop_began = Instant::now();
    session.stop();
    assert!(stop_began.elapsed() < Duration::from_millis(50));
}

#[test]
fn stop_returns_promptly_when_the_channel_is_full() {
    let (tx, _rx) = bounded(1);
    let session = FakeMediaSource::fast()
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let began = Instant::now();
    session.stop();
    assert!(began.elapsed() < Duration::from_millis(50));
}
