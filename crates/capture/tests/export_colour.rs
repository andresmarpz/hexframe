#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use greyscreen_capture::transcode::{self, Request};
use greyscreen_core::export::EncodePlan;
use greyscreen_core::media::VideoFormat;
use greyscreen_mp4::avc::{VuiColour, annexb_to_avcc, nal_type, parse_sps_colour, split_annexb};
use greyscreen_mp4::moov::{Chunk, Chunking, SampleRow, SampleTables, write_ftyp, write_full_moov};

const VIDEO: &[u8] = include_bytes!("../../mp4/tests/fixtures/v.h264");
const BT709_LIMITED: VuiColour = VuiColour {
    primaries: 1,
    transfer: 1,
    matrix: 1,
    full_range: false,
};

fn access_units(stream: &[u8]) -> Vec<Vec<u8>> {
    let mut units: Vec<Vec<u8>> = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut has_slice = false;
    for nal in split_annexb(stream) {
        let is_slice = matches!(nal_type(nal[0]), 1 | 5);
        if is_slice && has_slice {
            units.push(std::mem::take(&mut current));
            has_slice = false;
        }
        current.extend_from_slice(&[0, 0, 0, 1]);
        current.extend_from_slice(nal);
        has_slice |= is_slice;
    }
    units.push(current);
    units
}

fn source_clip() -> Vec<u8> {
    let (mut sps, mut pps) = (None, None);
    let samples: Vec<(Vec<u8>, bool)> = access_units(VIDEO)
        .iter()
        .map(|unit| {
            let (avcc, info) = annexb_to_avcc(unit);
            sps = sps.take().or(info.sps);
            pps = pps.take().or(info.pps);
            (avcc, info.idr)
        })
        .collect();
    let format = VideoFormat {
        width: 640,
        height: 360,
        fps: 60,
        sps: sps.unwrap_or_default(),
        pps: pps.unwrap_or_default(),
    };
    let head = write_ftyp(b"isom", &[*b"isom", *b"iso6", *b"avc1", *b"mp41"]);
    let payload_start = head.len() as u64 + 8;
    let mut payload = Vec::new();
    let mut tables = SampleTables::default();
    for (data, sync) in &samples {
        tables.video.push(SampleRow {
            offset: payload_start + payload.len() as u64,
            size: data.len() as u32,
            duration: format.frame_ticks(),
            sync: *sync,
        });
        payload.extend_from_slice(data);
    }
    let chunking = Chunking {
        video: tables
            .video
            .iter()
            .enumerate()
            .map(|(i, row)| Chunk {
                first_sample: i as u32,
                count: 1,
                offset: row.offset,
            })
            .collect(),
        audio: Vec::new(),
    };
    let moov = write_full_moov(&format, None, &tables, &chunking);
    let mut mdat = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    mdat.extend_from_slice(b"mdat");
    [head, mdat, payload, moov].concat()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn sps_of(file: &[u8]) -> Vec<u8> {
    let Some(at) = find(file, b"avcC") else {
        panic!("avcC box");
    };
    let length = usize::from(u16::from_be_bytes([file[at + 10], file[at + 11]]));
    file[at + 12..at + 12 + length].to_vec()
}

struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Self {
        Self(std::env::temp_dir().join(format!("greyscreen-{}-{name}.mp4", std::process::id())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).ok();
    }
}

#[test]
fn discord_export_is_tagged_bt709_limited_like_a_recording() {
    let source = Temp::new("export-colour-src");
    let export = Temp::new("export-colour-dst");
    std::fs::write(source.path(), source_clip()).expect("write the source clip");
    let request = Request {
        src: source.path(),
        dst: export.path(),
        start_secs: 0.0,
        end_secs: 1.0,
        plan: EncodePlan {
            width: 640,
            height: 360,
            fps: 60,
            video_bps: 2_000_000,
            audio_bps: 96_000,
        },
    };
    transcode::transcode(&request, &mut |_| {}, &AtomicBool::new(false)).expect("export");

    let bytes = std::fs::read(export.path()).expect("read the export");
    assert_eq!(parse_sps_colour(&sps_of(&bytes)), Some(BT709_LIMITED));
    if let Some(at) = find(&bytes, b"colr") {
        assert_eq!(&bytes[at + 4..at + 8], b"nclx");
        assert_eq!(&bytes[at + 8..at + 15], &[0, 1, 0, 1, 0, 1, 0]);
    } else {
        eprintln!("the MP4 sink wrote no colr box");
    }
}
