#[macro_use]
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_mp4::aac::asc_lc;
use greyscreen_mp4::avc::{annexb_to_avcc, nal_type, split_annexb};
use greyscreen_mp4::clip::write_clip;
use greyscreen_mp4::reader::boxes;

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

fn arrival(media: &Media, loops: usize, with_audio: bool) -> Vec<Arc<Packet>> {
    packets(media, loops, with_audio)
        .into_iter()
        .map(Arc::new)
        .collect()
}

fn video_index(all: &[Arc<Packet>], frame: usize) -> usize {
    all.iter()
        .position(|p| p.track == Track::Video && p.dts == frame as i64 * VIDEO_STEP)
        .unwrap_or_else(|| panic!("no video frame {frame}"))
}

fn slice(
    all: &[Arc<Packet>],
    from_frame: usize,
    to_frame: usize,
    lead_audio: usize,
) -> Vec<Arc<Packet>> {
    let idr = video_index(all, from_frame);
    let mut out: Vec<Arc<Packet>> = all[..idr]
        .iter()
        .filter(|p| p.track == Track::Audio)
        .rev()
        .take(lead_audio)
        .cloned()
        .collect();
    out.reverse();
    out.extend_from_slice(&all[idr..video_index(all, to_frame)]);
    out
}

fn audio_packets(path: &Path) -> usize {
    ffprobe(
        path,
        &["-select_streams", "a", "-show_entries", "packet=pts"],
    )
    .lines()
    .count()
}

fn expected_audio(slice: &[Arc<Packet>], idr_dts: i64) -> usize {
    slice
        .iter()
        .filter(|p| p.track == Track::Audio)
        .filter(|p| (p.dts + AUDIO_STEP) * 90_000 > idr_dts * 48_000)
        .count()
}

#[test]
fn clip_from_the_middle_starts_on_a_key_frame_and_decodes() {
    require_ffmpeg!();
    let media = media();
    let all = arrival(&media, 4, true);
    let part = &slice(&all, 120, 300, 5);
    let dir = TempDir::new("clip-middle");
    let path = dir.join("clip.mp4");
    write_clip(&path, &media.video_format, Some(&media.audio_format), part)
        .unwrap_or_else(|e| panic!("write_clip: {e}"));

    assert_streams(&path, true);
    assert_decodes_cleanly(&path);
    assert_eq!(video_packets(&path), (180, 3));
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
    assert_eq!(audio_packets(&path), expected_audio(part, 120 * VIDEO_STEP));
    assert!((format_duration(&path) - 3.0).abs() < 0.06);
    assert!(!dir.join("clip.mp4.tmp").exists());
}

#[test]
fn leading_audio_before_the_key_frame_is_dropped_except_the_covering_frame() {
    require_ffmpeg!();
    let media = media();
    let all = arrival(&media, 4, true);
    let part = &slice(&all, 120, 300, 8);
    let leading = part.iter().take_while(|p| p.track == Track::Audio).count();
    assert_eq!(leading, 8);
    let kept = expected_audio(part, 120 * VIDEO_STEP);
    let total = part.iter().filter(|p| p.track == Track::Audio).count();
    assert!(total - kept >= 5);

    let dir = TempDir::new("clip-lead");
    let path = dir.join("clip.mp4");
    write_clip(&path, &media.video_format, Some(&media.audio_format), part)
        .unwrap_or_else(|e| panic!("write_clip: {e}"));
    assert_eq!(audio_packets(&path), kept);
    assert_decodes_cleanly(&path);
}

#[test]
fn clip_is_faststart() {
    let media = media();
    let all = arrival(&media, 4, true);
    let dir = TempDir::new("clip-faststart");
    let path = dir.join("clip.mp4");
    write_clip(
        &path,
        &media.video_format,
        Some(&media.audio_format),
        &slice(&all, 60, 240, 0),
    )
    .unwrap_or_else(|e| panic!("write_clip: {e}"));
    let bytes = std::fs::read(&path).unwrap_or_default();
    assert_eq!(fourccs(&bytes), ["ftyp", "moov", "mdat"]);
}

#[test]
fn video_only_clip_plays() {
    require_ffmpeg!();
    let media = media();
    let all = arrival(&media, 4, false);
    let dir = TempDir::new("clip-video-only");
    let path = dir.join("clip.mp4");
    write_clip(&path, &media.video_format, None, &slice(&all, 60, 300, 0))
        .unwrap_or_else(|e| panic!("write_clip: {e}"));
    assert_streams(&path, false);
    assert_decodes_cleanly(&path);
    assert_eq!(video_packets(&path), (240, 4));
}

#[test]
fn rejects_packets_that_do_not_start_with_a_key_frame() {
    let media = media();
    let all = arrival(&media, 4, true);
    let dir = TempDir::new("clip-reject");
    let path = dir.join("clip.mp4");
    let from = video_index(&all, 121);
    let audio = Some(&media.audio_format);
    let result = write_clip(&path, &media.video_format, audio, &all[from..from + 50]);
    assert!(result.is_err());
    assert!(!path.exists());
    assert!(!dir.join("clip.mp4.tmp").exists());
    let audio_only: Vec<Arc<Packet>> = all
        .iter()
        .filter(|p| p.track == Track::Audio)
        .cloned()
        .collect();
    assert!(write_clip(&path, &media.video_format, audio, &audio_only).is_err());
    assert!(write_clip(&path, &media.video_format, None, &[]).is_err());
}
