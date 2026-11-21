use std::path::PathBuf;
use std::sync::Arc;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};

use super::*;
use crate::aac::asc_lc;
use crate::fragmented::FragmentedWriter;

const VIDEO_STEP: i64 = 1500;
const AUDIO_STEP: i64 = 1024;

fn video_format() -> VideoFormat {
    VideoFormat {
        width: 640,
        height: 360,
        fps: 60,
        sps: vec![0x67, 0x42, 0x00, 0x1f],
        pps: vec![0x68, 0xce, 0x3c, 0x80],
    }
}

fn audio_format() -> AudioFormat {
    AudioFormat {
        sample_rate: 48_000,
        channels: 2,
        bitrate: 192_000,
        asc: asc_lc(48_000, 2),
    }
}

fn video(tick: i64) -> Packet {
    Packet {
        track: Track::Video,
        dts: tick * VIDEO_STEP,
        pts: tick * VIDEO_STEP,
        duration: VIDEO_STEP as u32,
        key: tick % 60 == 0,
        data: Arc::from(vec![7u8; 100 + (tick % 7) as usize]),
    }
}

fn audio(index: i64) -> Packet {
    Packet {
        track: Track::Audio,
        dts: index * AUDIO_STEP,
        pts: index * AUDIO_STEP,
        duration: AUDIO_STEP as u32,
        key: true,
        data: Arc::from(vec![9u8; 50]),
    }
}

fn path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("greyscreen-inspect-{tag}-{}.mp4", std::process::id()))
}

fn write(
    path: &std::path::Path,
    ticks: impl Iterator<Item = i64>,
    audio_indices: impl Iterator<Item = i64>,
) -> FragmentedWriter {
    let mut writer =
        FragmentedWriter::create(path, &video_format(), Some(&audio_format())).expect("create");
    let mut audio_indices = audio_indices.peekable();
    for tick in ticks {
        writer.push(&video(tick)).expect("video");
        while audio_indices
            .peek()
            .is_some_and(|&i| i * AUDIO_STEP * 90_000 / 48_000 <= tick * VIDEO_STEP)
        {
            writer
                .push(&audio(audio_indices.next().expect("peeked")))
                .expect("audio");
        }
    }
    writer
}

#[test]
fn finalized_file_reports_every_sample_with_its_duration_and_keyframes() {
    let file = path("finalized");
    let writer = write(&file, (0..120).chain(180..240), 0..300);
    writer.finish().expect("finish");

    let media = inspect(&file).expect("inspect");
    std::fs::remove_file(&file).ok();
    assert_eq!(media.layout, Layout::Finalized);
    assert_eq!(
        (media.video.width, media.video.height, media.video.fps),
        (640, 360, 60)
    );
    assert_eq!(media.audio.expect("audio").sample_rate, 48_000);
    assert_eq!(media.tables.video.len(), 180);
    let long: Vec<u32> = media
        .tables
        .video
        .iter()
        .map(|r| r.duration)
        .filter(|&d| d != 1500)
        .collect();
    assert_eq!(long, [1500 * 61]);
    let keys: Vec<usize> = media
        .tables
        .video
        .iter()
        .enumerate()
        .filter(|(_, r)| r.sync)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(keys, [0, 60, 120]);
    assert!(
        media
            .tables
            .audio
            .iter()
            .all(|r| r.sync && r.duration == 1024)
    );
    assert_eq!(media.tables.audio.len(), 187);
    assert_eq!(media.tables.video[0].size, 100);
}

#[test]
fn an_unfinished_file_is_read_fragment_by_fragment() {
    let file = path("unfinished");
    let writer = write(&file, 0..200, 0..250);
    drop(writer);

    let media = inspect(&file).expect("inspect");
    let len = std::fs::metadata(&file).expect("meta").len();
    std::fs::remove_file(&file).ok();
    let Layout::Unfinalized {
        fragments,
        trailing_bytes,
    } = media.layout
    else {
        panic!("expected an unfinished file");
    };
    assert_eq!(fragments, 3);
    assert_eq!(trailing_bytes, 0);
    assert_eq!(media.bytes, len);
    assert_eq!(media.tables.video.len(), 180);
}

#[test]
fn a_cut_off_tail_is_reported_as_trailing_bytes() {
    let file = path("cut");
    let writer = write(&file, 0..130, 0..150);
    drop(writer);
    let len = std::fs::metadata(&file).expect("meta").len();
    let cut = std::fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .expect("open");
    cut.set_len(len - 40).expect("truncate");
    drop(cut);

    let media = inspect(&file).expect("inspect");
    std::fs::remove_file(&file).ok();
    let Layout::Unfinalized {
        fragments,
        trailing_bytes,
    } = media.layout
    else {
        panic!("expected an unfinished file");
    };
    assert_eq!(fragments, 1);
    assert!(trailing_bytes > 0);
}

#[test]
fn a_file_that_is_not_mp4_is_an_error() {
    let file = path("junk");
    std::fs::write(&file, b"not an mp4 at all").expect("write");
    let result = inspect(&file);
    std::fs::remove_file(&file).ok();
    assert!(result.is_err());
}

#[test]
fn sample_secs_sums_durations() {
    let rows = vec![
        SampleRow {
            offset: 0,
            size: 1,
            duration: 45_000,
            sync: true,
        },
        SampleRow {
            offset: 0,
            size: 1,
            duration: 45_000,
            sync: false,
        },
    ];
    assert_eq!(sample_secs(&rows, 90_000), 1.0);
}
