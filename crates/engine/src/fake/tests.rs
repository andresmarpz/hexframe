use std::time::{Duration, Instant};

use crossbeam_channel::bounded;

use super::*;

fn config() -> CaptureConfig {
    CaptureConfig {
        fps: 60,
        max_height: None,
        video_bitrate: 8_000_000,
        video_peak: 12_000_000,
        quality: None,
        gop_frames: 60,
        audio: true,
        audio_bitrate: 192_000,
        cursor: false,
        bitrate_pixels: 0,
        diagnostics: false,
    }
}

const STOP_LIMIT: Duration = Duration::from_secs(2);

fn collect_fast(seconds: i64) -> (Vec<MediaEvent>, Vec<Packet>) {
    collect(&FakeMediaSource::fast(), seconds)
}

fn collect(source: &FakeMediaSource, seconds: i64) -> (Vec<MediaEvent>, Vec<Packet>) {
    let (tx, rx) = bounded(256);
    let session = source
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
    let deadline = begin + Duration::from_secs(10);
    let target = i64::from(VideoFormat::TIMESCALE) * 3 / 10;
    let mut last_video = 0;
    while last_video < target && Instant::now() < deadline {
        if let Ok(MediaEvent::Packet(p)) = rx.recv_timeout(Duration::from_millis(50))
            && p.track == Track::Video
        {
            last_video = p.dts;
        }
    }
    let wall = begin.elapsed().as_secs_f64();
    let media = last_video as f64 / f64::from(VideoFormat::TIMESCALE);
    assert!(media >= 0.3, "media only reached {media} s in {wall} s");
    assert!(
        media <= wall + 0.1,
        "faster than real time: {media} s in {wall} s"
    );
    let stop_began = Instant::now();
    session.stop();
    assert!(stop_began.elapsed() < STOP_LIMIT);
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
    assert!(began.elapsed() < STOP_LIMIT);
}

fn start_error(source: &FakeMediaSource) -> Option<String> {
    let (tx, _rx) = bounded(1);
    source
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .err()
}

#[test]
fn fail_after_packets_sends_the_failure_after_that_many_packets() {
    let source = FakeMediaSource::fast().failing_after_packets(25, "boom");
    let (events, _) = collect(&source, 1);
    let failed = events
        .iter()
        .position(|e| matches!(e, MediaEvent::Failed(m) if m == "boom"))
        .expect("failure event");
    let packets_before = events[..failed]
        .iter()
        .filter(|e| matches!(e, MediaEvent::Packet(_)))
        .count();
    assert_eq!(packets_before, 25);
}

#[test]
fn fail_after_delay_sends_the_failure_once_the_delay_passed() {
    let source = FakeMediaSource::fast().failing_after(Duration::from_millis(50), "late");
    let (tx, rx) = bounded(4096);
    let began = Instant::now();
    let session = source
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    let failed_at = loop {
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(MediaEvent::Failed(message)) => {
                assert_eq!(message, "late");
                break began.elapsed();
            }
            Ok(_) => {}
            Err(_) => panic!("no failure event"),
        }
    };
    session.stop();
    assert!(failed_at >= Duration::from_millis(50), "{failed_at:?}");
}

#[test]
fn a_failing_start_returns_the_message_and_counts_no_session() {
    let source = FakeMediaSource::fast().failing_to_start("no encoder");
    assert_eq!(start_error(&source).as_deref(), Some("no encoder"));
    assert_eq!(source.probe().live_sessions(), 0);
}

#[test]
fn a_blocking_start_stalls_the_caller() {
    let source = FakeMediaSource::fast().blocking_start(Duration::from_millis(300));
    let (tx, _rx) = bounded(1);
    let began = Instant::now();
    let session = source
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    assert!(began.elapsed() >= Duration::from_millis(300));
    session.stop();
}

#[test]
fn a_blocking_stop_is_cut_at_the_stop_bound() {
    let source = FakeMediaSource::fast()
        .blocking_stop(Duration::from_secs(30))
        .with_stop_bound(Duration::from_millis(300));
    let (tx, _rx) = bounded(1);
    let session = source
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    let began = Instant::now();
    session.stop();
    let took = began.elapsed();
    assert!(took >= Duration::from_millis(300), "{took:?}");
    assert!(took < STOP_LIMIT, "{took:?}");
}

#[test]
fn audio_can_start_after_video() {
    let source = FakeMediaSource::fast().with_audio_delay_secs(2);
    let (_, packets) = collect(&source, 3);
    let first_video = of(&packets, Track::Video)[0].dts;
    let first_audio = of(&packets, Track::Audio)[0].dts;
    assert_eq!(first_video, 0);
    assert_eq!(first_audio, 2 * i64::from(SAMPLE_RATE));
}

#[test]
fn audio_can_stop_with_a_warning() {
    let source = FakeMediaSource::fast().stopping_audio_after(10, "audio device lost");
    let (events, packets) = collect(&source, 2);
    assert_eq!(of(&packets, Track::Audio).len(), 10);
    assert!(of(&packets, Track::Video).len() > 100);
    let warnings: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            MediaEvent::Warning(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(warnings, ["audio device lost"]);
}

#[test]
fn a_stats_event_can_be_the_last_event_before_stop() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let session = FakeMediaSource::new()
        .with_final_stats()
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    session.stop();
    let events: Vec<_> = rx.try_iter().collect();
    assert!(matches!(events.last(), Some(MediaEvent::Stats(_))));
    let stats = events
        .iter()
        .filter(|e| matches!(e, MediaEvent::Stats(_)))
        .count();
    assert_eq!(stats, 1);
}

#[test]
fn the_probe_counts_live_sessions() {
    let source = FakeMediaSource::fast();
    let probe = source.probe();
    let (tx, _rx) = bounded(1);
    let first = source
        .start(CaptureTarget::PrimaryMonitor, config(), tx.clone())
        .unwrap();
    let second = source
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    assert_eq!(probe.live_sessions(), 2);
    first.stop();
    assert_eq!(probe.live_sessions(), 1);
    second.stop();
    assert_eq!(probe.live_sessions(), 0);
}

#[test]
fn the_probe_records_the_longest_send_wait() {
    let source = FakeMediaSource::fast();
    let probe = source.probe();
    let (tx, _rx) = bounded(1);
    let session = source
        .start(CaptureTarget::PrimaryMonitor, config(), tx)
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    session.stop();
    assert!(probe.max_send_wait() >= Duration::from_millis(100));
}
