#[macro_use]
mod common;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_mp4::Mp4Error;
use greyscreen_mp4::aac::asc_lc;
use greyscreen_mp4::avc::{annexb_to_avcc, nal_type, split_annexb};
use greyscreen_mp4::finalize::placeholder_bytes;
use greyscreen_mp4::fragmented::FragmentedWriter;
use greyscreen_mp4::reader::boxes;

use common::faulty::FaultySink;

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

struct TempPath(PathBuf);

impl TempPath {
    fn new(name: &str) -> Self {
        Self(std::env::temp_dir().join(format!("greyscreen-fmp4-{}-{name}.mp4", std::process::id())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).ok();
    }
}

fn write_file(path: &Path, media: &Media, packets: &[Packet], with_audio: bool, finish: bool) {
    let audio = with_audio.then_some(&media.audio_format);
    let mut writer = FragmentedWriter::create(path, &media.video_format, audio)
        .unwrap_or_else(|e| panic!("create: {e}"));
    for packet in packets {
        writer.push(packet).unwrap_or_else(|e| panic!("push: {e}"));
    }
    if finish {
        let info = writer.finish().unwrap_or_else(|e| panic!("finish: {e}"));
        let len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        assert_eq!(info.bytes, len);
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

#[test]
fn finished_file_plays_and_decodes() {
    require_ffmpeg!();
    let media = media();
    let file = TempPath::new("finished");
    let packets = packets(&media, 1, true);
    let audio = Some(&media.audio_format);
    let mut writer = FragmentedWriter::create(file.path(), &media.video_format, audio)
        .unwrap_or_else(|e| panic!("create: {e}"));
    for packet in &packets {
        writer.push(packet).unwrap_or_else(|e| panic!("push: {e}"));
    }
    let info = writer.finish().unwrap_or_else(|e| panic!("finish: {e}"));

    assert_streams(file.path(), true);
    let probed = format_duration(file.path());
    assert!((probed - 2.0).abs() < 0.05, "duration {probed}");
    assert!((info.duration_secs - probed).abs() < 0.05);
    assert_eq!(video_packets(file.path()), (120, 2));
    assert_decodes_cleanly(file.path());

    let bytes = std::fs::read(file.path()).unwrap_or_default();
    assert_eq!(info.bytes, bytes.len() as u64);
    assert_eq!(fourccs(&bytes), ["ftyp", "free", "mdat", "moov"]);
}

#[test]
fn many_fragments_finish_with_exact_sample_counts() {
    require_ffmpeg!();
    let media = media();
    let file = TempPath::new("many");
    write_file(file.path(), &media, &packets(&media, 4, true), true, true);

    assert_streams(file.path(), true);
    let probed = format_duration(file.path());
    let audio_secs = (4 * media.audio.len() * 1024) as f64 / 48_000.0;
    let expected = audio_secs.max(8.0);
    assert!((probed - expected).abs() < 0.05, "duration {probed}");
    assert_eq!(video_packets(file.path()), (480, 8));
    assert_decodes_cleanly(file.path());
}

#[test]
fn video_only_file_plays() {
    require_ffmpeg!();
    let media = media();
    let file = TempPath::new("video-only");
    write_file(file.path(), &media, &packets(&media, 2, false), false, true);

    assert_streams(file.path(), false);
    assert_eq!(video_packets(file.path()), (240, 4));
    assert_decodes_cleanly(file.path());
}

#[test]
fn unfinished_file_is_valid_fragmented_mp4() {
    require_ffmpeg!();
    let media = media();
    let file = TempPath::new("crash");
    write_file(file.path(), &media, &packets(&media, 4, true), true, false);

    let bytes = std::fs::read(file.path()).unwrap_or_default();
    let order = fourccs(&bytes);
    assert_eq!(&order[..3], ["ftyp", "free", "moov"]);
    let rest = &order[3..];
    assert_eq!(rest.len(), 14);
    assert!(rest.chunks(2).all(|pair| pair == ["moof", "mdat"]));
    assert_streams(file.path(), true);
}

#[test]
fn file_truncated_after_third_fragment_still_reads() {
    require_ffmpeg!();
    let media = media();
    let file = TempPath::new("truncate-src");
    write_file(file.path(), &media, &packets(&media, 4, true), true, false);

    let bytes = std::fs::read(file.path()).unwrap_or_default();
    let third_mdat_end = boxes(&bytes)
        .filter(|b| &b.fourcc == b"mdat")
        .nth(2)
        .map_or(0, |b| b.end());
    assert!(third_mdat_end > 0 && third_mdat_end < bytes.len());

    let cut = TempPath::new("truncate-cut");
    std::fs::write(cut.path(), &bytes[..third_mdat_end])
        .unwrap_or_else(|e| panic!("write cut: {e}"));
    assert_streams(cut.path(), true);
    assert_eq!(video_packets(cut.path()), (180, 3));
    assert_decodes_cleanly(cut.path());
}

#[test]
fn rejects_stream_that_does_not_start_on_a_key_frame() {
    let media = media();
    let file = TempPath::new("no-key");
    let mut writer = FragmentedWriter::create(file.path(), &media.video_format, None)
        .unwrap_or_else(|e| panic!("create: {e}"));
    let mut packet = packets(&media, 1, false).remove(0);
    packet.key = false;
    assert!(writer.push(&packet).is_err());
}

#[test]
fn placeholder_switches_to_64_bit_mdat_past_4_gib() {
    let small = placeholder_bytes(28, 1_000_000).unwrap_or_default();
    assert_eq!(&small[0..8], [0, 0, 0, 8, b'f', b'r', b'e', b'e']);
    assert_eq!(&small[12..16], b"mdat");
    assert_eq!(
        u32::from_be_bytes([small[8], small[9], small[10], small[11]]),
        1_000_000 - 36
    );

    let moov_offset = 5_000_000_000u64;
    let large = placeholder_bytes(28, moov_offset).unwrap_or_default();
    assert_eq!(&large[0..8], [0, 0, 0, 1, b'm', b'd', b'a', b't']);
    let size = u64::from_be_bytes(large[8..16].try_into().unwrap_or_default());
    assert_eq!(size, moov_offset - 28);
}

#[test]
fn file_is_rebased_to_the_first_video_key_frame() {
    require_ffmpeg!();
    let media = media();
    let file = TempPath::new("rebase");
    let mut writer =
        FragmentedWriter::create(file.path(), &media.video_format, Some(&media.audio_format))
            .unwrap_or_else(|e| panic!("create: {e}"));
    let shift = 3000;
    let mut all = packets(&media, 2, true);
    for packet in all.iter_mut().filter(|p| p.track == Track::Video) {
        packet.dts += shift;
        packet.pts += shift;
    }
    all.sort_by(|a, b| {
        let secs = |p: &Packet| {
            let rate = if p.track == Track::Video {
                90_000.0
            } else {
                48_000.0
            };
            p.dts as f64 / rate
        };
        secs(a).total_cmp(&secs(b))
    });
    for packet in &all {
        writer.push(packet).unwrap_or_else(|e| panic!("push: {e}"));
    }
    writer.finish().unwrap_or_else(|e| panic!("finish: {e}"));

    let first_pts = |stream: &str| -> f64 {
        let out = ffprobe(
            file.path(),
            &[
                "-select_streams",
                stream,
                "-show_entries",
                "packet=pts_time",
                "-read_intervals",
                "%+#1",
            ],
        );
        field(&out, "packets.packet.0.pts_time")
            .parse()
            .unwrap_or(f64::NAN)
    };
    let video = first_pts("v");
    let audio = first_pts("a");
    eprintln!("first video {video}, first audio {audio}");
    assert!(video.abs() < 1e-6);
    assert!(audio.abs() <= 0.021);
    assert_decodes_cleanly(file.path());
}

fn muxed(media: &Media, packets: &[Packet], name: &str) -> Vec<u8> {
    let file = TempPath::new(name);
    write_file(file.path(), media, packets, true, true);
    std::fs::read(file.path()).unwrap_or_else(|e| panic!("read: {e}"))
}

fn shifted(packets: &[Packet], video_ticks: i64, audio_ticks: i64) -> Vec<Packet> {
    packets
        .iter()
        .map(|p| {
            let by = if p.track == Track::Video {
                video_ticks
            } else {
                audio_ticks
            };
            Packet {
                dts: p.dts + by,
                pts: p.pts + by,
                ..p.clone()
            }
        })
        .collect()
}

fn audio_sample_count(file: &[u8]) -> u32 {
    let moov = boxes(file)
        .find(|b| &b.fourcc == b"moov")
        .unwrap_or_else(|| panic!("no moov"));
    let audio_track = moov
        .children(file)
        .filter(|b| &b.fourcc == b"trak")
        .nth(1)
        .unwrap_or_else(|| panic!("no audio track"));
    let stsz = [b"mdia", b"minf", b"stbl", b"stsz"]
        .iter()
        .try_fold(audio_track, |parent, name| parent.find(file, name))
        .unwrap_or_else(|| panic!("no stsz"));
    let payload = stsz.payload_bytes(file);
    u32::from_be_bytes(payload[8..12].try_into().unwrap_or_default())
}

#[test]
fn streams_that_start_late_mux_exactly_like_streams_that_start_at_zero() {
    let media = media();
    let packets = packets(&media, 2, true);
    let from_zero = muxed(&media, &packets, "from-zero");

    for (name, video, audio) in [
        ("late-short", 90_000 * 7, 48_000 * 7),
        ("late-long", 90_000 * 3_600, 48_000 * 3_600),
    ] {
        let late = muxed(&media, &shifted(&packets, video, audio), name);

        assert!(late == from_zero, "{name} differs from the zero start");
    }
}

#[test]
fn audio_that_ends_before_the_first_key_frame_is_dropped_by_absolute_time() {
    let media = media();
    let packets = packets(&media, 2, true);
    let all_audio = audio_sample_count(&muxed(&media, &packets, "audio-all"));
    let mut ordered = shifted(&packets, 90_000, 0);
    ordered.sort_by(|a, b| {
        let rate = |p: &Packet| {
            if p.track == Track::Video {
                90_000.0
            } else {
                48_000.0
            }
        };
        (a.dts as f64 / rate(a)).total_cmp(&(b.dts as f64 / rate(b)))
    });

    let kept = audio_sample_count(&muxed(&media, &ordered, "audio-late-video"));

    assert_eq!(all_audio - kept, 46);
}

#[test]
fn write_failure_is_returned_by_the_first_push_that_hits_it() {
    let media = media();
    let packets = packets(&media, 3, false);
    let probe = FaultySink::healthy();
    let head = probe.written();
    FragmentedWriter::with_sink(probe, &media.video_format, None).expect("create");
    let head_len = head.load(Ordering::SeqCst);
    let second_key = packets
        .iter()
        .enumerate()
        .filter(|(_, p)| p.key)
        .nth(1)
        .map(|(i, _)| i)
        .expect("second key frame");
    for kind in [ErrorKind::StorageFull, ErrorKind::Other] {
        let sink = FaultySink::new(head_len + 1, kind);
        let mut writer =
            FragmentedWriter::with_sink(sink, &media.video_format, None).expect("create");
        let (index, err) = packets
            .iter()
            .enumerate()
            .find_map(|(i, packet)| writer.push(packet).err().map(|e| (i, e)))
            .expect("a push fails");
        assert_eq!(index, second_key);
        match err {
            Mp4Error::Io(io) => assert_eq!(io.kind(), kind),
            other => panic!("unexpected error {other}"),
        }
    }
}

#[test]
fn healthy_sink_writes_the_same_bytes_as_a_real_file() {
    let media = media();
    let packets = packets(&media, 3, true);
    let file = TempPath::new("sink-identical");
    write_file(file.path(), &media, &packets, true, true);
    let sink = FaultySink::healthy();
    let contents = sink.contents();
    let mut writer =
        FragmentedWriter::with_sink(sink, &media.video_format, Some(&media.audio_format))
            .expect("create");
    for packet in &packets {
        writer.push(packet).expect("push");
    }
    writer.finish().expect("finish");
    let on_disk = std::fs::read(file.path()).expect("read");
    let in_memory = contents.lock().expect("lock").get_ref().clone();
    assert!(!on_disk.is_empty());
    assert!(on_disk == in_memory, "sink bytes differ from the file");
}
