use super::*;
use crate::avc::{NAL_PPS, NAL_SPS, nal_type, split_annexb};
use crate::reader::{BoxRef, boxes};

const FIXTURE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/v.h264"
));

fn video() -> VideoFormat {
    let nal = |kind| {
        split_annexb(FIXTURE)
            .find(|n| nal_type(n[0]) == kind)
            .map(<[u8]>::to_vec)
            .unwrap_or_default()
    };
    VideoFormat {
        width: 640,
        height: 360,
        fps: 60,
        sps: nal(NAL_SPS),
        pps: nal(NAL_PPS),
    }
}

fn audio() -> AudioFormat {
    AudioFormat {
        sample_rate: 48_000,
        channels: 2,
        bitrate: 192_000,
        asc: vec![0x11, 0x90],
    }
}

fn child(data: &[u8], parent: &BoxRef, fourcc: &[u8; 4]) -> BoxRef {
    parent
        .find(data, fourcc)
        .unwrap_or_else(|| panic!("missing {}", String::from_utf8_lossy(fourcc)))
}

fn traks(data: &[u8], moov: &BoxRef) -> Vec<BoxRef> {
    moov.children(data)
        .filter(|b| &b.fourcc == b"trak")
        .collect()
}

fn stbl(data: &[u8], trak: &BoxRef) -> BoxRef {
    let mdia = child(data, trak, b"mdia");
    let minf = child(data, &mdia, b"minf");
    child(data, &minf, b"stbl")
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn u32s(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks(4).map(|c| u32_at(c, 0)).collect()
}

fn root(data: &[u8]) -> BoxRef {
    boxes(data).next().unwrap_or_else(|| panic!("no moov"))
}

fn table(data: &[u8], stbl: &BoxRef, fourcc: &[u8; 4]) -> Vec<u32> {
    let b = child(data, stbl, fourcc);
    u32s(&b.payload_bytes(data)[4..])
}

#[test]
fn ftyp_lists_brands() {
    let out = write_ftyp(b"isom", &[*b"isom", *b"iso6", *b"avc1", *b"mp41"]);
    assert_eq!(out.len(), 8 + 8 + 16);
    assert_eq!(&out[4..12], b"ftypisom");
    assert_eq!(u32_at(&out, 12), 0x200);
    assert_eq!(&out[16..], b"isomiso6avc1mp41");
}

#[test]
fn init_moov_has_empty_tables_and_mvex() {
    let out = write_init_moov(&video(), Some(&audio()));
    let moov = root(&out);
    assert_eq!(&moov.fourcc, b"moov");
    assert!(moov.complete && moov.end() == out.len());

    let mvhd = child(&out, &moov, b"mvhd");
    let mvhd = mvhd.payload_bytes(&out);
    assert_eq!(u32_at(mvhd, 12), 1000);
    assert_eq!(u32_at(mvhd, 16), 0);
    assert_eq!(u32_at(mvhd, 96), 3);

    let tracks = traks(&out, &moov);
    assert_eq!(tracks.len(), 2);
    for trak in &tracks {
        let stbl = stbl(&out, trak);
        let names: Vec<[u8; 4]> = stbl.children(&out).map(|b| b.fourcc).collect();
        assert_eq!(names, [*b"stsd", *b"stts", *b"stsc", *b"stsz", *b"stco"]);
        assert_eq!(table(&out, &stbl, b"stts"), [0]);
        assert_eq!(table(&out, &stbl, b"stsc"), [0]);
        assert_eq!(table(&out, &stbl, b"stco"), [0]);
    }

    let mvex = child(&out, &moov, b"mvex");
    let trex: Vec<BoxRef> = mvex.children(&out).collect();
    assert_eq!(trex.len(), 2);
    let flags = |b: &BoxRef| u32s(&b.payload_bytes(&out)[4..]);
    assert_eq!(flags(&trex[0]), [1, 1, 0, 0, 0x0101_0000]);
    assert_eq!(flags(&trex[1]), [2, 1, 0, 0, 0x0200_0000]);
}

#[test]
fn init_moov_sample_entries_carry_codec_config() {
    let v = video();
    let a = audio();
    let out = write_init_moov(&v, Some(&a));
    let moov = root(&out);
    let tracks = traks(&out, &moov);

    let stsd = child(&out, &stbl(&out, &tracks[0]), b"stsd");
    let avc1 = first_after(&out, &stsd, 8);
    assert_eq!(&avc1.fourcc, b"avc1");
    let entry = avc1.payload_bytes(&out);
    assert_eq!(u16::from_be_bytes([entry[24], entry[25]]), 640);
    assert_eq!(u16::from_be_bytes([entry[26], entry[27]]), 360);
    let nested: Vec<[u8; 4]> = boxes_after(&out, &avc1, 78).map(|b| b.fourcc).collect();
    assert_eq!(nested, [*b"avcC", *b"colr", *b"pasp"]);
    let colr = boxes_after(&out, &avc1, 78)
        .nth(1)
        .unwrap_or_else(|| panic!("colr"));
    assert_eq!(&colr.payload_bytes(&out)[..4], b"nclx");
    assert_eq!(&colr.payload_bytes(&out)[4..], &[0, 1, 0, 1, 0, 1, 0]);

    let stsd = child(&out, &stbl(&out, &tracks[1]), b"stsd");
    let mp4a = first_after(&out, &stsd, 8);
    assert_eq!(&mp4a.fourcc, b"mp4a");
    let entry = mp4a.payload_bytes(&out);
    assert_eq!(u16::from_be_bytes([entry[16], entry[17]]), 2);
    assert_eq!(u16::from_be_bytes([entry[18], entry[19]]), 16);
    assert_eq!(u32_at(entry, 24), 48_000 << 16);
    let esds = boxes_after(&out, &mp4a, 28)
        .next()
        .unwrap_or_else(|| panic!("esds"));
    assert_eq!(&esds.fourcc, b"esds");
    let body = esds.payload_bytes(&out);
    assert!(body.windows(4).any(|w| w == [0x05, 0x02, 0x11, 0x90]));
}

fn boxes_after<'a>(data: &'a [u8], parent: &BoxRef, skip: usize) -> crate::reader::Boxes<'a> {
    crate::reader::Boxes::within(data, parent.payload.start + skip..parent.payload.end)
}

fn first_after(data: &[u8], parent: &BoxRef, skip: usize) -> BoxRef {
    boxes_after(data, parent, skip)
        .next()
        .unwrap_or_else(|| panic!("no entry"))
}

#[test]
fn init_moov_without_audio_has_one_track() {
    let out = write_init_moov(&video(), None);
    let moov = root(&out);
    assert_eq!(traks(&out, &moov).len(), 1);
    assert_eq!(
        child(&out, &child(&out, &moov, b"mvex"), b"trex").size,
        8 + 4 + 20
    );
}

fn rows(offset: u64, spec: &[(u32, u32, bool)]) -> Vec<SampleRow> {
    let mut at = offset;
    spec.iter()
        .map(|&(size, duration, sync)| {
            let row = SampleRow {
                offset: at,
                size,
                duration,
                sync,
            };
            at += u64::from(size);
            row
        })
        .collect()
}

fn sample_tables() -> (SampleTables, Chunking) {
    let video_rows = rows(
        1000,
        &[
            (10, 1500, true),
            (11, 1500, false),
            (12, 1500, false),
            (13, 3000, true),
            (14, 3000, false),
        ],
    );
    let audio_rows = rows(
        5000,
        &[(20, 1024, true), (21, 1024, true), (22, 1024, true)],
    );
    let chunking = Chunking {
        video: vec![
            Chunk {
                first_sample: 0,
                count: 2,
                offset: 1000,
            },
            Chunk {
                first_sample: 2,
                count: 2,
                offset: 1021,
            },
            Chunk {
                first_sample: 4,
                count: 1,
                offset: 1046,
            },
        ],
        audio: vec![
            Chunk {
                first_sample: 0,
                count: 2,
                offset: 5000,
            },
            Chunk {
                first_sample: 2,
                count: 1,
                offset: 5041,
            },
        ],
    };
    (
        SampleTables {
            video: video_rows,
            audio: audio_rows,
        },
        chunking,
    )
}

#[test]
fn full_moov_describes_samples_and_chunks() {
    let (tables, chunking) = sample_tables();
    let out = write_full_moov(&video(), Some(&audio()), &tables, &chunking);
    let moov = root(&out);
    assert!(moov.find(&out, b"mvex").is_none());

    let mvhd = child(&out, &moov, b"mvhd");
    assert_eq!(u32_at(mvhd.payload_bytes(&out), 16), 116);

    let tracks = traks(&out, &moov);
    let v = stbl(&out, &tracks[0]);
    assert_eq!(table(&out, &v, b"stts"), [2, 3, 1500, 2, 3000]);
    assert_eq!(table(&out, &v, b"stss"), [2, 1, 4]);
    assert_eq!(table(&out, &v, b"stsc"), [2, 1, 2, 1, 3, 1, 1]);
    assert_eq!(table(&out, &v, b"stsz"), [0, 5, 10, 11, 12, 13, 14]);
    let co64 = child(&out, &v, b"co64");
    let offsets: Vec<u64> = co64.payload_bytes(&out)[8..]
        .chunks(8)
        .map(|c| u64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect();
    assert_eq!(offsets, [1000, 1021, 1046]);

    let a = stbl(&out, &tracks[1]);
    assert!(a.find(&out, b"stss").is_none());
    assert_eq!(table(&out, &a, b"stts"), [1, 3, 1024]);
    assert_eq!(table(&out, &a, b"stsc"), [2, 1, 2, 1, 2, 1, 1]);
    assert_eq!(table(&out, &a, b"stsz"), [0, 3, 20, 21, 22]);
}

#[test]
fn full_moov_track_durations_use_their_own_timescales() {
    let (tables, chunking) = sample_tables();
    let out = write_full_moov(&video(), Some(&audio()), &tables, &chunking);
    let moov = root(&out);
    let tracks = traks(&out, &moov);

    let expect = [(10_500u32, 116u32, 90_000u32), (3072, 64, 48_000)];
    for (trak, (ticks, movie_ms, timescale)) in tracks.iter().zip(expect) {
        let tkhd = child(&out, trak, b"tkhd");
        assert_eq!(u32_at(tkhd.payload_bytes(&out), 20), movie_ms);
        let mdia = child(&out, trak, b"mdia");
        let mdhd = child(&out, &mdia, b"mdhd");
        let mdhd = mdhd.payload_bytes(&out);
        assert_eq!((u32_at(mdhd, 12), u32_at(mdhd, 16)), (timescale, ticks));
    }
}

#[test]
fn full_moov_size_does_not_depend_on_offsets() {
    let (tables, chunking) = sample_tables();
    let a = write_full_moov(&video(), Some(&audio()), &tables, &chunking);

    let shift = |rows: &[SampleRow]| -> Vec<SampleRow> {
        rows.iter()
            .map(|r| SampleRow {
                offset: r.offset + 7_000_000_000,
                ..*r
            })
            .collect()
    };
    let shifted_chunks = |c: &[Chunk]| -> Vec<Chunk> {
        c.iter()
            .map(|c| Chunk {
                offset: c.offset + 7_000_000_000,
                ..*c
            })
            .collect()
    };
    let b = write_full_moov(
        &video(),
        Some(&audio()),
        &SampleTables {
            video: shift(&tables.video),
            audio: shift(&tables.audio),
        },
        &Chunking {
            video: shifted_chunks(&chunking.video),
            audio: shifted_chunks(&chunking.audio),
        },
    );
    assert_eq!(a.len(), b.len());
    assert_ne!(a, b);
}

#[test]
fn per_sample_chunks_follow_row_offsets() {
    let (tables, _) = sample_tables();
    let chunks = chunk_per_sample(&tables.video);
    assert_eq!(chunks.len(), 5);
    assert_eq!(
        chunks[3],
        Chunk {
            first_sample: 3,
            count: 1,
            offset: 1033
        }
    );

    let out = write_full_moov(
        &video(),
        None,
        &tables,
        &Chunking {
            video: chunks,
            audio: Vec::new(),
        },
    );
    let moov = root(&out);
    let v = stbl(&out, &traks(&out, &moov)[0]);
    assert_eq!(table(&out, &v, b"stsc"), [1, 1, 1, 1]);
}

#[test]
fn audio_delay_becomes_a_leading_empty_edit() {
    let (tables, chunking) = sample_tables();
    let start = AudioStart {
        delay_ms: 3000,
        skip_samples: 0,
    };
    let out = write_full_moov_with_start(&video(), Some(&audio()), &tables, &chunking, start);
    let moov = root(&out);
    let tracks = traks(&out, &moov);

    assert!(tracks[0].find(&out, b"edts").is_none());
    let edts = child(&out, &tracks[1], b"edts");
    let elst = child(&out, &edts, b"elst");
    let payload = elst.payload_bytes(&out);
    assert_eq!(&payload[..4], [0, 0, 0, 0]);
    assert_eq!(u32_at(payload, 4), 2);
    let entry = |n: usize| {
        let at = 8 + n * 12;
        (
            u32_at(payload, at),
            u32_at(payload, at + 4) as i32,
            u32_at(payload, at + 8),
        )
    };
    assert_eq!(entry(0), (3000, -1, 0x0001_0000));
    assert_eq!(entry(1), (64, 0, 0x0001_0000));

    let audio_tkhd = child(&out, &tracks[1], b"tkhd");
    assert_eq!(u32_at(audio_tkhd.payload_bytes(&out), 20), 3064);
    let mvhd = child(&out, &moov, b"mvhd");
    assert_eq!(u32_at(mvhd.payload_bytes(&out), 16), 3064);
}

#[test]
fn no_audio_delay_writes_no_edit_list() {
    let (tables, chunking) = sample_tables();
    let out = write_full_moov(&video(), Some(&audio()), &tables, &chunking);
    let moov = root(&out);
    for trak in traks(&out, &moov) {
        assert!(trak.find(&out, b"edts").is_none());
    }
}
