use std::path::{Path, PathBuf};
use std::process::Command;

use greyscreen_core::media::{AudioFormat, VideoFormat};
use greyscreen_mp4::aac::asc_lc;
use greyscreen_mp4::avc::{annexb_to_avcc, nal_type, split_annexb};
use greyscreen_mp4::moov::{
    Chunk, Chunking, SampleRow, SampleTables, chunk_per_sample, write_ftyp, write_full_moov,
    write_init_moov,
};
use greyscreen_mp4::reader::boxes;

const VIDEO: &[u8] = include_bytes!("fixtures/v.h264");
const AUDIO: &[u8] = include_bytes!("fixtures/a.adts");
const BRANDS: [[u8; 4]; 4] = [*b"isom", *b"iso6", *b"avc1", *b"mp41"];

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

struct Layout {
    payload: Vec<u8>,
    tables: SampleTables,
    video_chunks: Vec<Chunk>,
    audio_chunks: Vec<Chunk>,
}

fn layout(media: &Media, payload_start: u64) -> Layout {
    let mut payload = Vec::new();
    let mut tables = SampleTables::default();
    for (data, sync) in &media.video {
        tables.video.push(SampleRow {
            offset: payload_start + payload.len() as u64,
            size: data.len() as u32,
            duration: media.video_format.frame_ticks(),
            sync: *sync,
        });
        payload.extend_from_slice(data);
    }
    for data in &media.audio {
        tables.audio.push(SampleRow {
            offset: payload_start + payload.len() as u64,
            size: data.len() as u32,
            duration: 1024,
            sync: true,
        });
        payload.extend_from_slice(data);
    }
    let group = |rows: &[SampleRow], per_chunk: usize| -> Vec<Chunk> {
        rows.chunks(per_chunk)
            .enumerate()
            .map(|(i, rows)| Chunk {
                first_sample: (i * per_chunk) as u32,
                count: rows.len() as u32,
                offset: rows[0].offset,
            })
            .collect()
    };
    Layout {
        video_chunks: group(&tables.video, 60),
        audio_chunks: group(&tables.audio, 16),
        payload,
        tables,
    }
}

fn mdat(payload: &[u8]) -> Vec<u8> {
    let mut out = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend_from_slice(b"mdat");
    out.extend_from_slice(payload);
    out
}

fn moov_for(media: &Media, layout: &Layout, per_sample: bool) -> Vec<u8> {
    let chunking = if per_sample {
        Chunking {
            video: chunk_per_sample(&layout.tables.video),
            audio: chunk_per_sample(&layout.tables.audio),
        }
    } else {
        Chunking {
            video: layout.video_chunks.clone(),
            audio: layout.audio_chunks.clone(),
        }
    };
    write_full_moov(
        &media.video_format,
        Some(&media.audio_format),
        &layout.tables,
        &chunking,
    )
}

fn ftyp() -> Vec<u8> {
    write_ftyp(b"isom", &BRANDS)
}

fn moov_at_end(media: &Media, per_sample: bool) -> Vec<u8> {
    let mut file = ftyp();
    let layout = layout(media, file.len() as u64 + 8);
    file.extend_from_slice(&mdat(&layout.payload));
    file.extend_from_slice(&moov_for(media, &layout, per_sample));
    file
}

fn faststart(media: &Media) -> Vec<u8> {
    let head = ftyp();
    let moov_len = moov_for(media, &layout(media, 0), false).len();
    let layout = layout(media, (head.len() + moov_len + 8) as u64);
    let moov = moov_for(media, &layout, false);
    assert_eq!(moov.len(), moov_len);
    [head, moov, mdat(&layout.payload)].concat()
}

fn tool_available(name: &str) -> bool {
    Command::new(name).arg("-version").output().is_ok()
}

struct TempFile(PathBuf);

impl TempFile {
    fn new(name: &str, bytes: &[u8]) -> Self {
        let path =
            std::env::temp_dir().join(format!("greyscreen-mp4-{}-{name}.mp4", std::process::id()));
        std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).ok();
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

fn verify_playable(file: &TempFile) {
    let streams = ffprobe(
        file.path(),
        &["-show_entries", "stream=codec_name,width,height,duration"],
    );
    assert_eq!(field(&streams, "streams.stream.0.codec_name"), "h264");
    assert_eq!(field(&streams, "streams.stream.0.width"), "640");
    assert_eq!(field(&streams, "streams.stream.0.height"), "360");
    assert_eq!(field(&streams, "streams.stream.1.codec_name"), "aac");
    for stream in 0..2 {
        let duration: f64 = field(&streams, &format!("streams.stream.{stream}.duration"))
            .parse()
            .unwrap_or(0.0);
        assert!(
            (duration - 2.0).abs() < 0.05,
            "stream {stream} duration {duration}"
        );
    }

    let packets = ffprobe(
        file.path(),
        &["-select_streams", "v", "-show_entries", "packet=flags"],
    );
    let keys = packets.lines().filter(|l| l.ends_with("=\"K__\"")).count();
    let total = packets.lines().count();
    assert_eq!((total, keys), (120, 2));

    let decode = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(file.path())
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

macro_rules! require_ffmpeg {
    () => {
        if !(tool_available("ffprobe") && tool_available("ffmpeg")) {
            eprintln!("skipped: ffprobe or ffmpeg is not on PATH");
            return;
        }
    };
}

#[test]
fn moov_at_end_file_plays_and_decodes() {
    require_ffmpeg!();
    let file = TempFile::new("end", &moov_at_end(&media(), false));
    verify_playable(&file);
}

#[test]
fn faststart_file_plays_and_decodes() {
    require_ffmpeg!();
    let bytes = faststart(&media());
    let order: Vec<[u8; 4]> = boxes(&bytes).map(|b| b.fourcc).collect();
    assert_eq!(order, [*b"ftyp", *b"moov", *b"mdat"]);
    verify_playable(&TempFile::new("fast", &bytes));
}

#[test]
fn one_chunk_per_sample_file_plays_and_decodes() {
    require_ffmpeg!();
    let file = TempFile::new("persample", &moov_at_end(&media(), true));
    verify_playable(&file);
}

#[test]
fn init_moov_alone_is_a_valid_empty_file() {
    require_ffmpeg!();
    let media = media();
    let bytes = [
        ftyp(),
        write_init_moov(&media.video_format, Some(&media.audio_format)),
    ]
    .concat();
    let file = TempFile::new("init", &bytes);
    let streams = ffprobe(
        file.path(),
        &["-show_entries", "stream=codec_name,width,height"],
    );
    assert_eq!(field(&streams, "streams.stream.0.codec_name"), "h264");
    assert_eq!(field(&streams, "streams.stream.0.width"), "640");
    assert_eq!(field(&streams, "streams.stream.1.codec_name"), "aac");
}

#[test]
fn video_only_file_plays() {
    require_ffmpeg!();
    let media = media();
    let mut file = ftyp();
    let layout = layout(&media, file.len() as u64 + 8);
    let video_len: usize = media.video.iter().map(|(d, _)| d.len()).sum();
    file.extend_from_slice(&mdat(&layout.payload[..video_len]));
    let tables = SampleTables {
        video: layout.tables.video.clone(),
        audio: Vec::new(),
    };
    let chunking = Chunking {
        video: layout.video_chunks.clone(),
        audio: Vec::new(),
    };
    file.extend_from_slice(&write_full_moov(
        &media.video_format,
        None,
        &tables,
        &chunking,
    ));
    let temp = TempFile::new("video-only", &file);
    let streams = ffprobe(
        temp.path(),
        &["-show_entries", "stream=codec_name,duration"],
    );
    assert_eq!(field(&streams, "streams.stream.0.codec_name"), "h264");
    assert!(!streams.contains("streams.stream.1"));
}
