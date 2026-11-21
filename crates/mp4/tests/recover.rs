#[macro_use]
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_mp4::aac::asc_lc;
use greyscreen_mp4::avc::{annexb_to_avcc, nal_type, split_annexb};
use greyscreen_mp4::fragmented::FragmentedWriter;
use greyscreen_mp4::reader::boxes;
use greyscreen_mp4::recover::recover;

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

fn format_duration(path: &Path) -> f64 {
    let format = ffprobe(path, &["-show_entries", "format=duration"]);
    field(&format, "format.duration").parse().unwrap_or(0.0)
}

fn fourccs(bytes: &[u8]) -> Vec<String> {
    boxes(bytes)
        .map(|b| String::from_utf8_lossy(&b.fourcc).into_owned())
        .collect()
}

fn write_part(dir: &TempDir, media: &Media, loops: usize, with_audio: bool) -> PathBuf {
    let path = dir.join("game.mp4.part");
    let audio = with_audio.then_some(&media.audio_format);
    let mut writer = FragmentedWriter::create(&path, &media.video_format, audio)
        .unwrap_or_else(|e| panic!("create: {e}"));
    for packet in &packets(media, loops, with_audio) {
        writer.push(packet).unwrap_or_else(|e| panic!("push: {e}"));
    }
    drop(writer);
    path
}

fn truncate_to(path: &Path, len: usize) {
    let bytes = std::fs::read(path).unwrap_or_default();
    std::fs::write(path, &bytes[..len]).unwrap_or_else(|e| panic!("truncate: {e}"));
}

fn box_starts(path: &Path, fourcc: &[u8; 4]) -> Vec<usize> {
    let bytes = std::fs::read(path).unwrap_or_default();
    boxes(&bytes)
        .filter(|b| &b.fourcc == fourcc)
        .map(|b| b.start)
        .collect()
}

fn recovered_ok(dir: &TempDir, expected_frames: usize, expected_keys: usize, with_audio: bool) {
    let out = recover(&dir.join("game.mp4.part")).unwrap_or_else(|e| panic!("recover: {e}"));
    assert_eq!(out, dir.join("game.mp4"));
    assert!(!dir.join("game.mp4.part").exists());
    assert_streams(&out, with_audio);
    assert_decodes_cleanly(&out);
    assert_eq!(video_packets(&out), (expected_frames, expected_keys));
    let bytes = std::fs::read(&out).unwrap_or_default();
    assert_eq!(fourccs(&bytes)[..3], ["ftyp", "free", "mdat"]);
}

#[test]
fn dropped_writer_output_recovers_and_plays() {
    require_ffmpeg!();
    let media = media();
    let dir = TempDir::new("recover-drop");
    write_part(&dir, &media, 4, true);
    let out = recover(&dir.join("game.mp4.part")).unwrap_or_else(|e| panic!("recover: {e}"));
    assert_eq!(out, dir.join("game.mp4"));
    assert!(!dir.join("game.mp4.part").exists());
    assert_streams(&out, true);
    assert_decodes_cleanly(&out);
    assert!((format_duration(&out) - 8.0).abs() <= 1.05);
    let (frames, keys) = video_packets(&out);
    assert_eq!((frames, keys), (420, 7));
    let bytes = std::fs::read(&out).unwrap_or_default();
    let order = fourccs(&bytes);
    assert_eq!(order[..3], ["ftyp", "free", "mdat"]);
    assert_eq!(order.last().map(String::as_str), Some("moov"));
}

#[test]
fn file_cut_in_the_middle_of_an_mdat_recovers_earlier_fragments() {
    require_ffmpeg!();
    let media = media();
    let dir = TempDir::new("recover-cut-mdat");
    let part = write_part(&dir, &media, 4, true);
    let third_mdat = box_starts(&part, b"mdat")[2];
    truncate_to(&part, third_mdat + 500);
    recovered_ok(&dir, 120, 2, true);
}

#[test]
fn file_cut_in_the_middle_of_a_moof_recovers_earlier_fragments() {
    require_ffmpeg!();
    let media = media();
    let dir = TempDir::new("recover-cut-moof");
    let part = write_part(&dir, &media, 4, true);
    let third_moof = box_starts(&part, b"moof")[2];
    truncate_to(&part, third_moof + 40);
    recovered_ok(&dir, 120, 2, true);
}

#[test]
fn video_only_file_recovers() {
    require_ffmpeg!();
    let media = media();
    let dir = TempDir::new("recover-video-only");
    let part = write_part(&dir, &media, 3, false);
    let second_mdat = box_starts(&part, b"mdat")[1];
    truncate_to(&part, second_mdat + 100);
    recovered_ok(&dir, 60, 1, false);
}

#[test]
fn file_without_a_complete_fragment_is_kept_as_recovered_fmp4() {
    let media = media();
    let dir = TempDir::new("recover-none");
    let part = write_part(&dir, &media, 2, true);
    let first_moof = box_starts(&part, b"moof")[0];
    truncate_to(&part, first_moof + 30);
    assert!(recover(&part).is_err());
    assert!(!part.exists());
    assert!(dir.join("game.mp4.recovered-fmp4").exists());
    assert!(!dir.join("game.mp4").exists());
}

#[test]
fn junk_file_is_kept_as_recovered_fmp4() {
    let dir = TempDir::new("recover-junk");
    let part = dir.join("game.mp4.part");
    std::fs::write(&part, [7u8; 100]).unwrap_or_else(|e| panic!("write: {e}"));
    assert!(recover(&part).is_err());
    assert!(dir.join("game.mp4.recovered-fmp4").exists());
}

#[test]
fn rejects_a_file_that_is_not_a_part_file() {
    let dir = TempDir::new("recover-name");
    let other = dir.join("game.mp4");
    std::fs::write(&other, [0u8; 10]).unwrap_or_else(|e| panic!("write: {e}"));
    assert!(recover(&other).is_err());
    assert!(other.exists());
}

#[test]
fn finalized_part_is_only_renamed() {
    require_ffmpeg!();
    let media = media();
    let dir = TempDir::new("recover-finalized");
    let part = dir.join("game.mp4.part");
    let mut writer =
        FragmentedWriter::create(&part, &media.video_format, Some(&media.audio_format))
            .unwrap_or_else(|e| panic!("create: {e}"));
    for packet in &packets(&media, 2, true) {
        writer.push(packet).unwrap_or_else(|e| panic!("push: {e}"));
    }
    writer.finish().unwrap_or_else(|e| panic!("finish: {e}"));
    let before = std::fs::read(&part).unwrap_or_default();
    let out = recover(&part).unwrap_or_else(|e| panic!("recover: {e}"));
    assert_eq!(out, dir.join("game.mp4"));
    assert!(!part.exists());
    assert_eq!(std::fs::read(&out).unwrap_or_default(), before);
    assert_streams(&out, true);
    assert_decodes_cleanly(&out);
}
