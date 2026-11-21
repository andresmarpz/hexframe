use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_mp4::aac::asc_lc;
use greyscreen_mp4::avc::{annexb_to_avcc, nal_type, split_annexb};
use greyscreen_mp4::clip::write_clip;
use greyscreen_mp4::demux::Movie;
use greyscreen_mp4::reader::boxes;
use greyscreen_mp4::trim::trim;

const VIDEO: &[u8] = include_bytes!("fixtures/v.h264");
const AUDIO: &[u8] = include_bytes!("fixtures/a.adts");
const VIDEO_STEP: i64 = 1500;
const AUDIO_STEP: i64 = 1024;

struct Media {
    video_format: VideoFormat,
    audio_format: AudioFormat,
    video: Vec<(Vec<u8>, bool)>,
    audio: Vec<Vec<u8>>,
}

fn access_units(stream: &[u8]) -> Vec<Vec<u8>> {
    let mut units: Vec<Vec<u8>> = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut has_slice = false;
    let mut pending: Vec<u8> = Vec::new();
    for nal in split_annexb(stream) {
        let is_slice = matches!(nal_type(nal[0]), 1 | 5);
        if is_slice && has_slice {
            units.push(std::mem::take(&mut current));
            current.append(&mut pending);
            has_slice = false;
        }
        let target = if is_slice || !has_slice {
            &mut current
        } else {
            &mut pending
        };
        target.extend_from_slice(&[0, 0, 0, 1]);
        target.extend_from_slice(nal);
        has_slice |= is_slice;
    }
    units.push(current);
    units
}

fn strip_adts(stream: &[u8]) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut pos = 0;
    while pos + 7 <= stream.len() {
        let header = if stream[pos + 1] & 1 == 0 { 9 } else { 7 };
        let len = (usize::from(stream[pos + 3] & 3) << 11)
            | (usize::from(stream[pos + 4]) << 3)
            | usize::from(stream[pos + 5] >> 5);
        frames.push(stream[pos + header..pos + len].to_vec());
        pos += len;
    }
    frames
}

fn media() -> Media {
    let mut sps = None;
    let mut pps = None;
    let video: Vec<(Vec<u8>, bool)> = access_units(VIDEO)
        .iter()
        .map(|unit| {
            let (avcc, info) = annexb_to_avcc(unit);
            sps = sps.take().or(info.sps);
            pps = pps.take().or(info.pps);
            (avcc, info.idr)
        })
        .collect();
    Media {
        video_format: VideoFormat {
            width: 640,
            height: 360,
            fps: 60,
            sps: sps.unwrap_or_default(),
            pps: pps.unwrap_or_default(),
        },
        audio_format: AudioFormat {
            sample_rate: 48_000,
            channels: 2,
            bitrate: 192_000,
            asc: asc_lc(48_000, 2),
        },
        video,
        audio: strip_adts(AUDIO),
    }
}

fn packets(media: &Media, loops: usize, with_audio: bool) -> Vec<Packet> {
    let mut timed: Vec<(f64, Packet)> = Vec::new();
    for i in 0..loops * media.video.len() {
        let (data, key) = &media.video[i % media.video.len()];
        let dts = i as i64 * VIDEO_STEP;
        timed.push((
            dts as f64 / 90_000.0,
            Packet {
                track: Track::Video,
                dts,
                pts: dts,
                duration: VIDEO_STEP as u32,
                key: *key,
                data: Arc::from(data.as_slice()),
            },
        ));
    }
    if with_audio {
        for i in 0..loops * media.audio.len() {
            let dts = i as i64 * AUDIO_STEP;
            timed.push((
                dts as f64 / 48_000.0 - 1e-9,
                Packet {
                    track: Track::Audio,
                    dts,
                    pts: dts,
                    duration: AUDIO_STEP as u32,
                    key: true,
                    data: Arc::from(media.audio[i % media.audio.len()].as_slice()),
                },
            ));
        }
    }
    timed.sort_by(|a, b| a.0.total_cmp(&b.0));
    timed.into_iter().map(|(_, p)| p).collect()
}
fn tool_available(name: &str) -> bool {
    Command::new(name).arg("-version").output().is_ok()
}

macro_rules! require_ffmpeg {
    () => {
        if !(tool_available("ffprobe") && tool_available("ffmpeg")) {
            eprintln!("skipped: ffprobe or ffmpeg is not on PATH");
            return;
        }
    };
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("greyscreen-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir: {e}"));
        Self(dir)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn ffprobe(path: &Path, args: &[&str]) -> String {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-of", "flat"])
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("ffprobe: {e}"));
    assert!(
        out.stderr.is_empty(),
        "ffprobe stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn field(flat: &str, key: &str) -> String {
    flat.lines()
        .find_map(|l| l.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("no {key} in:\n{flat}"))
        .trim_matches('"')
        .to_owned()
}

fn assert_decodes_cleanly(path: &Path) {
    let decode = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .output()
        .unwrap_or_else(|e| panic!("ffmpeg: {e}"));
    assert!(decode.status.success());
    assert!(
        decode.stderr.is_empty(),
        "decode errors: {}",
        String::from_utf8_lossy(&decode.stderr)
    );
}

fn video_packets(path: &Path) -> (usize, usize) {
    let out = ffprobe(
        path,
        &["-select_streams", "v", "-show_entries", "packet=flags"],
    );
    let keys = out.lines().filter(|l| l.ends_with("=\"K__\"")).count();
    (out.lines().count(), keys)
}

fn assert_streams(path: &Path, expect_audio: bool) {
    let streams = ffprobe(path, &["-show_entries", "stream=codec_name,width,height"]);
    assert_eq!(field(&streams, "streams.stream.0.codec_name"), "h264");
    assert_eq!(field(&streams, "streams.stream.0.width"), "640");
    assert_eq!(field(&streams, "streams.stream.0.height"), "360");
    if expect_audio {
        assert_eq!(field(&streams, "streams.stream.1.codec_name"), "aac");
    } else {
        assert!(!streams.contains("streams.stream.1"));
    }
}

fn audio_packets(path: &Path) -> usize {
    ffprobe(
        path,
        &["-select_streams", "a", "-show_entries", "packet=pts"],
    )
    .lines()
    .count()
}

fn duration(path: &Path) -> f64 {
    let format = ffprobe(path, &["-show_entries", "format=duration"]);
    field(&format, "format.duration").parse().unwrap_or(0.0)
}

fn top_level(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read: {e}"));
    boxes(&bytes)
        .map(|b| String::from_utf8_lossy(&b.fourcc).into_owned())
        .collect()
}

fn source(dir: &TempDir, name: &str, seconds: usize) -> PathBuf {
    let media = media();
    let all: Vec<Arc<Packet>> = packets(&media, seconds / 2, true)
        .into_iter()
        .map(Arc::new)
        .collect();
    let path = dir.join(name);
    write_clip(&path, &media.video_format, Some(&media.audio_format), &all)
        .unwrap_or_else(|e| panic!("write source: {e}"));
    path
}

fn trimmed(dir: &TempDir, from: &Path, name: &str, start: f64, end: f64) -> PathBuf {
    let path = dir.join(name);
    trim(from, &path, start, end).unwrap_or_else(|e| panic!("trim: {e}"));
    path
}

#[test]
fn the_demuxer_reads_back_what_the_clip_writer_wrote() {
    let dir = TempDir::new("trim-demux");
    let media = media();
    let path = source(&dir, "source.mp4", 4);
    let movie = Movie::open(&path).unwrap_or_else(|e| panic!("open: {e}"));
    assert_eq!(movie.video, media.video_format);
    assert_eq!(movie.audio.as_ref(), Some(&media.audio_format));
    assert_eq!(movie.video_samples.len(), 4 * media.video.len() / 2);
    let keys: Vec<usize> = movie
        .video_samples
        .iter()
        .enumerate()
        .filter(|(_, s)| s.key)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(keys, vec![0, 60, 120, 180]);
    assert!(movie.audio_samples.iter().all(|s| s.key));
    assert!((movie.video_secs() - 4.0).abs() < 1e-9);
    let mut reader = movie.reader().unwrap_or_else(|e| panic!("reader: {e}"));
    let first = reader
        .read(&movie.video_samples[0])
        .unwrap_or_else(|e| panic!("read: {e}"));
    assert_eq!(first, media.video[0].0);
}

#[test]
fn trim_starts_on_the_key_frame_before_the_start_and_reports_the_snapped_range() {
    require_ffmpeg!();
    let dir = TempDir::new("trim-trim");
    let from = source(&dir, "source.mp4", 8);
    let path = dir.join("cut.mp4");
    let range = trim(&from, &path, 1.5, 4.2).unwrap_or_else(|e| panic!("trim: {e}"));

    assert_eq!(range.start_secs, 1.0);
    assert!((range.end_secs - 4.2).abs() < 1e-9);
    assert!((range.duration_secs() - 3.2).abs() < 1e-9);
    assert_streams(&path, true);
    assert_decodes_cleanly(&path);
    assert_eq!(video_packets(&path), (192, 4));
    let first = ffprobe(
        &path,
        &[
            "-select_streams",
            "v",
            "-show_frames",
            "-read_intervals",
            "%+#1",
            "-show_entries",
            "frame=key_frame,pts",
        ],
    );
    assert_eq!(field(&first, "frames.frame.0.key_frame"), "1");
    assert_eq!(field(&first, "frames.frame.0.pts"), "0");
    assert!((duration(&path) - 3.2).abs() < 0.06);
    assert!(audio_packets(&path) >= (3.2 * 48_000.0 / 1024.0) as usize);
    assert_eq!(top_level(&path), ["ftyp", "moov", "mdat"]);
}

#[test]
fn trim_of_a_whole_file_keeps_every_frame() {
    require_ffmpeg!();
    let dir = TempDir::new("trim-trim-all");
    let from = source(&dir, "source.mp4", 4);
    let path = dir.join("all.mp4");
    let range = trim(&from, &path, 0.0, 100.0).unwrap_or_else(|e| panic!("trim: {e}"));
    assert_eq!((range.start_secs, range.end_secs), (0.0, 4.0));
    assert_eq!(video_packets(&path), (240, 4));
    assert_decodes_cleanly(&path);
}

#[test]
fn trim_can_cut_a_clip_that_was_already_cut() {
    require_ffmpeg!();
    let dir = TempDir::new("trim-trim-twice");
    let from = source(&dir, "source.mp4", 8);
    let once = trimmed(&dir, &from, "once.mp4", 1.0, 6.0);
    let twice = dir.join("twice.mp4");
    let range = trim(&once, &twice, 2.5, 4.0).unwrap_or_else(|e| panic!("trim: {e}"));
    assert_eq!((range.start_secs, range.end_secs), (2.0, 4.0));
    assert_eq!(video_packets(&twice), (120, 2));
    assert_decodes_cleanly(&twice);
}

#[test]
fn trim_refuses_a_range_without_video() {
    let dir = TempDir::new("trim-trim-empty");
    let from = source(&dir, "source.mp4", 4);
    assert!(trim(&from, &dir.join("nope.mp4"), 3.0, 3.0).is_err());
    assert!(trim(&from, &dir.join("nope.mp4"), 3.0, 2.0).is_err());
    assert!(!dir.join("nope.mp4").exists());
}

#[test]
fn trim_reports_the_key_frame_it_snapped_to_without_decoding() {
    let dir = TempDir::new("trim-snap");
    let from = source(&dir, "source.mp4", 8);
    let cut = |start: f64, end: f64| {
        let range =
            trim(&from, &dir.join("cut.mp4"), start, end).unwrap_or_else(|e| panic!("trim: {e}"));
        (range.start_secs, range.end_secs)
    };
    assert_eq!(cut(0.0, 1.0), (0.0, 1.0));
    assert_eq!(cut(0.99, 3.0), (0.0, 3.0));
    assert_eq!(cut(1.0, 3.0), (1.0, 3.0));
    assert_eq!(cut(1.01, 3.0), (1.0, 3.0));
    assert_eq!(cut(7.5, 100.0), (7.0, 8.0));
    let movie = Movie::open(&dir.join("cut.mp4")).unwrap_or_else(|e| panic!("open: {e}"));
    assert!(movie.video_samples[0].key);
    assert_eq!(movie.video_samples.len(), 60);
}
