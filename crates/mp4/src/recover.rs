use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::{Path, PathBuf};

use greyscreen_core::media::{AudioFormat, VideoFormat};

use crate::finalize::finalize;
use crate::moov::{
    AUDIO_TRACK_ID, Chunk, Chunking, NON_SYNC_SAMPLE_FLAGS, SYNC_SAMPLE_FLAGS, SampleRow,
    SampleTables, VIDEO_TRACK_ID,
};
use crate::reader::{BoxRef, Boxes, boxes};
use crate::{Mp4Error, Result};

const PLACEHOLDER_LEN: u64 = 16;
const FINALIZED_FREE_LEN: u64 = 8;
const IS_NON_SYNC: u32 = 0x1_0000;
const TFHD_BASE_DATA_OFFSET: u32 = 0x1;
const TFHD_SAMPLE_DESCRIPTION: u32 = 0x2;
const TFHD_DEFAULT_DURATION: u32 = 0x8;
const TFHD_DEFAULT_SIZE: u32 = 0x10;
const TFHD_DEFAULT_FLAGS: u32 = 0x20;
const TRUN_DATA_OFFSET: u32 = 0x1;
const TRUN_FIRST_SAMPLE_FLAGS: u32 = 0x4;
const TRUN_DURATION: u32 = 0x100;
const TRUN_SIZE: u32 = 0x200;
const TRUN_FLAGS: u32 = 0x400;
const TRUN_COMPOSITION_OFFSET: u32 = 0x800;
const AVC1_HEADER_LEN: usize = 78;
const MP4A_HEADER_LEN: usize = 28;

#[derive(Debug, Clone, Copy)]
struct Header {
    fourcc: [u8; 4],
    size: u64,
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    offset: u64,
    size: u32,
    duration: u32,
    sync: bool,
}

#[derive(Debug)]
struct Run {
    track_id: u32,
    samples: Vec<Sample>,
}

struct Scan {
    placeholder_offset: u64,
    init_moov: Vec<u8>,
    runs: Vec<Run>,
    good_end: u64,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let out = self.bytes.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(out)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u32_if(&mut self, present: bool) -> Option<Option<u32>> {
        if present {
            self.u32().map(Some)
        } else {
            Some(None)
        }
    }

    fn u64(&mut self) -> Option<u64> {
        let b: [u8; 8] = self.take(8)?.try_into().ok()?;
        Some(u64::from_be_bytes(b))
    }

    fn rest(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }
}

pub fn recover(part: &Path) -> Result<PathBuf> {
    let stem = part
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".part"))
        .filter(|s| s.ends_with(".mp4"))
        .ok_or_else(|| Mp4Error::Invalid("not a .mp4.part file".into()))?
        .to_owned();
    let finished = part.with_file_name(&stem);
    if already_finalized(part)? {
        std::fs::rename(part, &finished)?;
        return Ok(finished);
    }
    match rebuild(part) {
        Ok(()) => {
            std::fs::rename(part, &finished)?;
            Ok(finished)
        }
        Err(error) => {
            std::fs::rename(part, part.with_file_name(format!("{stem}.recovered-fmp4"))).ok();
            Err(error)
        }
    }
}

fn already_finalized(part: &Path) -> Result<bool> {
    let mut file = File::open(part)?;
    let len = file.metadata()?.len();
    let Some(ftyp) = header_at(&mut file, 0, len)?.filter(|h| &h.fourcc == b"ftyp") else {
        return Ok(false);
    };
    let Some(next) = header_at(&mut file, ftyp.size, len)? else {
        return Ok(false);
    };
    if &next.fourcc == b"mdat" {
        return Ok(true);
    }
    if &next.fourcc != b"free" || next.size != FINALIZED_FREE_LEN {
        return Ok(false);
    }
    Ok(header_at(&mut file, ftyp.size + next.size, len)?.is_some_and(|h| &h.fourcc == b"mdat"))
}

fn rebuild(part: &Path) -> Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(part)?;
    let scan = scan(&mut file)?;
    file.set_len(scan.good_end)?;
    let (video, audio) = read_formats(&scan.init_moov, &scan.runs)?;
    let (tables, chunking) = build_tables(&scan.runs);
    if tables.video.is_empty() {
        return Err(Mp4Error::Invalid("no complete fragment to recover".into()));
    }
    finalize(
        &mut file,
        scan.placeholder_offset,
        &video,
        audio.as_ref(),
        &tables,
        &chunking,
    )?;
    Ok(())
}

fn header_at(file: &mut File, pos: u64, len: u64) -> Result<Option<Header>> {
    if pos + 8 > len {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(pos))?;
    let mut head = [0u8; 8];
    file.read_exact(&mut head)?;
    let fourcc = [head[4], head[5], head[6], head[7]];
    let (size, header_len) = match u32::from_be_bytes([head[0], head[1], head[2], head[3]]) {
        0 => (len - pos, 8),
        1 => {
            if pos + 16 > len {
                return Ok(None);
            }
            let mut large = [0u8; 8];
            file.read_exact(&mut large)?;
            (u64::from_be_bytes(large), 16)
        }
        n => (u64::from(n), 8),
    };
    Ok((size >= header_len).then_some(Header { fourcc, size }))
}

fn read_exact_at(file: &mut File, pos: u64, len: u64) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(pos))?;
    let mut buf =
        vec![0u8; usize::try_from(len).map_err(|_| Mp4Error::Invalid("box too large".into()))?];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

fn scan(file: &mut File) -> Result<Scan> {
    let len = file.metadata()?.len();
    let expect = |header: Option<Header>, fourcc: &[u8; 4], pos: u64| {
        header
            .filter(|h| &h.fourcc == fourcc && pos + h.size <= len)
            .ok_or_else(|| {
                Mp4Error::Invalid(format!(
                    "missing complete {} box at {pos}",
                    String::from_utf8_lossy(fourcc)
                ))
            })
    };

    let ftyp = expect(header_at(file, 0, len)?, b"ftyp", 0)?;
    let placeholder_offset = ftyp.size;
    let free = expect(
        header_at(file, placeholder_offset, len)?,
        b"free",
        placeholder_offset,
    )?;
    if free.size != PLACEHOLDER_LEN {
        return Err(Mp4Error::Invalid("placeholder is not 16 bytes".into()));
    }
    let moov_offset = placeholder_offset + PLACEHOLDER_LEN;
    let moov = expect(header_at(file, moov_offset, len)?, b"moov", moov_offset)?;
    let init_moov = read_exact_at(file, moov_offset, moov.size)?;

    let mut pos = moov_offset + moov.size;
    let mut runs = Vec::new();
    let mut good_end = pos;
    while let Some(moof) = header_at(file, pos, len)? {
        if &moof.fourcc != b"moof" || pos + moof.size > len {
            break;
        }
        let mdat_pos = pos + moof.size;
        let Some(mdat) = header_at(file, mdat_pos, len)? else {
            break;
        };
        if &mdat.fourcc != b"mdat" || mdat_pos + mdat.size > len {
            break;
        }
        let moof_bytes = read_exact_at(file, pos, moof.size)?;
        let payload = mdat_pos + 8..mdat_pos + mdat.size;
        let Some(mut fragment) = parse_moof(&moof_bytes, pos, &payload) else {
            break;
        };
        runs.append(&mut fragment);
        pos = mdat_pos + mdat.size;
        good_end = pos;
    }
    Ok(Scan {
        placeholder_offset,
        init_moov,
        runs,
        good_end,
    })
}

fn parse_moof(data: &[u8], moof_start: u64, mdat_payload: &Range<u64>) -> Option<Vec<Run>> {
    let moof = boxes(data).next()?;
    let mut runs = Vec::new();
    for traf in moof.children(data).filter(|b| &b.fourcc == b"traf") {
        let tfhd = Tfhd::parse(traf.find(data, b"tfhd")?.payload_bytes(data))?;
        let mut next_offset = tfhd.base_data_offset.unwrap_or(moof_start);
        for trun in traf.children(data).filter(|b| &b.fourcc == b"trun") {
            let samples = parse_trun(trun.payload_bytes(data), &tfhd, &mut next_offset)?;
            if samples.iter().any(|s| {
                s.offset < mdat_payload.start || s.offset + u64::from(s.size) > mdat_payload.end
            }) {
                return None;
            }
            runs.push(Run {
                track_id: tfhd.track_id,
                samples,
            });
        }
    }
    Some(runs)
}

struct Tfhd {
    track_id: u32,
    base_data_offset: Option<u64>,
    default_duration: Option<u32>,
    default_size: Option<u32>,
    default_flags: u32,
}

impl Tfhd {
    fn parse(payload: &[u8]) -> Option<Self> {
        let mut cur = Cursor::new(payload);
        let flags = cur.u32()? & 0xFF_FFFF;
        let track_id = cur.u32()?;
        let base_data_offset = if flags & TFHD_BASE_DATA_OFFSET != 0 {
            Some(cur.u64()?)
        } else {
            None
        };
        cur.u32_if(flags & TFHD_SAMPLE_DESCRIPTION != 0)?;
        let default_duration = cur.u32_if(flags & TFHD_DEFAULT_DURATION != 0)?;
        let default_size = cur.u32_if(flags & TFHD_DEFAULT_SIZE != 0)?;
        let fallback_flags = if track_id == AUDIO_TRACK_ID {
            SYNC_SAMPLE_FLAGS
        } else {
            NON_SYNC_SAMPLE_FLAGS
        };
        let default_flags = cur
            .u32_if(flags & TFHD_DEFAULT_FLAGS != 0)?
            .unwrap_or(fallback_flags);
        Some(Self {
            track_id,
            base_data_offset,
            default_duration,
            default_size,
            default_flags,
        })
    }
}

fn parse_trun(payload: &[u8], tfhd: &Tfhd, next_offset: &mut u64) -> Option<Vec<Sample>> {
    let mut cur = Cursor::new(payload);
    let flags = cur.u32()? & 0xFF_FFFF;
    let count = cur.u32()?;
    let mut offset = if flags & TRUN_DATA_OFFSET != 0 {
        let base = tfhd.base_data_offset.unwrap_or(*next_offset);
        let relative = i64::from(cur.u32()? as i32);
        u64::try_from(i64::try_from(base).ok()?.checked_add(relative)?).ok()?
    } else {
        *next_offset
    };
    let first_flags = cur.u32_if(flags & TRUN_FIRST_SAMPLE_FLAGS != 0)?;
    let mut samples = Vec::new();
    for index in 0..count {
        let duration = cur
            .u32_if(flags & TRUN_DURATION != 0)?
            .or(tfhd.default_duration)?;
        let size = cur.u32_if(flags & TRUN_SIZE != 0)?.or(tfhd.default_size)?;
        let first = first_flags.filter(|_| index == 0);
        let sample_flags = cur
            .u32_if(flags & TRUN_FLAGS != 0)?
            .or(first)
            .unwrap_or(tfhd.default_flags);
        cur.u32_if(flags & TRUN_COMPOSITION_OFFSET != 0)?;
        samples.push(Sample {
            offset,
            size,
            duration,
            sync: sample_flags & IS_NON_SYNC == 0,
        });
        offset += u64::from(size);
    }
    *next_offset = offset;
    Some(samples)
}

fn build_tables(runs: &[Run]) -> (SampleTables, Chunking) {
    let mut tables = SampleTables::default();
    let mut chunking = Chunking::default();
    for run in runs {
        let (rows, chunks) = match run.track_id {
            VIDEO_TRACK_ID => (&mut tables.video, &mut chunking.video),
            AUDIO_TRACK_ID => (&mut tables.audio, &mut chunking.audio),
            _ => continue,
        };
        let Some(first) = run.samples.first() else {
            continue;
        };
        chunks.push(Chunk {
            first_sample: rows.len() as u32,
            count: run.samples.len() as u32,
            offset: first.offset,
        });
        rows.extend(run.samples.iter().map(|s| SampleRow {
            offset: s.offset,
            size: s.size,
            duration: s.duration,
            sync: s.sync,
        }));
    }
    (tables, chunking)
}

fn invalid(what: &str) -> Mp4Error {
    Mp4Error::Invalid(format!("init moov: {what}"))
}

fn child_path(data: &[u8], start: BoxRef, path: &[&[u8; 4]]) -> Option<BoxRef> {
    path.iter()
        .try_fold(start, |parent, fourcc| parent.find(data, fourcc))
}

fn read_formats(init: &[u8], runs: &[Run]) -> Result<(VideoFormat, Option<AudioFormat>)> {
    let moov = boxes(init)
        .find(|b| &b.fourcc == b"moov")
        .ok_or_else(|| invalid("no moov"))?;
    let mut video = None;
    let mut audio = None;
    for trak in moov.children(init).filter(|b| &b.fourcc == b"trak") {
        let stbl = child_path(init, trak.clone(), &[b"mdia", b"minf", b"stbl"])
            .ok_or_else(|| invalid("no stbl"))?;
        let stsd = stbl.find(init, b"stsd").ok_or_else(|| invalid("no stsd"))?;
        let entries = Boxes::within(init, stsd.payload.start + 8..stsd.payload.end);
        for entry in entries {
            match &entry.fourcc {
                b"avc1" => video = Some(parse_avc1(init, &entry)?),
                b"mp4a" => {
                    let mdhd = child_path(init, trak.clone(), &[b"mdia", b"mdhd"])
                        .ok_or_else(|| invalid("no mdhd"))?;
                    audio = Some(parse_mp4a(init, &entry, &mdhd)?);
                }
                _ => {}
            }
        }
    }
    let mut video = video.ok_or_else(|| invalid("no video track"))?;
    let first_duration = runs
        .iter()
        .find(|r| r.track_id == VIDEO_TRACK_ID)
        .and_then(|r| r.samples.first())
        .map_or(0, |s| s.duration);
    video.fps = (VideoFormat::TIMESCALE + first_duration / 2)
        .checked_div(first_duration)
        .unwrap_or(0)
        .max(1);
    Ok((video, audio))
}

fn parse_avc1(data: &[u8], entry: &BoxRef) -> Result<VideoFormat> {
    let payload = entry.payload_bytes(data);
    let mut head = Cursor::new(payload);
    head.take(24).ok_or_else(|| invalid("short avc1"))?;
    let width = head.u16().ok_or_else(|| invalid("short avc1"))?;
    let height = head.u16().ok_or_else(|| invalid("short avc1"))?;
    let avcc = Boxes::within(
        data,
        entry.payload.start + AVC1_HEADER_LEN..entry.payload.end,
    )
    .find(|b| &b.fourcc == b"avcC")
    .ok_or_else(|| invalid("no avcC"))?;
    let mut cur = Cursor::new(avcc.payload_bytes(data));
    let short = || invalid("short avcC");
    cur.take(5).ok_or_else(short)?;
    let sps_count = cur.u8().ok_or_else(short)? & 0x1F;
    let sps = first_nal(&mut cur, sps_count).ok_or_else(short)?;
    let pps_count = cur.u8().ok_or_else(short)?;
    let pps = first_nal(&mut cur, pps_count).ok_or_else(short)?;
    Ok(VideoFormat {
        width: u32::from(width),
        height: u32::from(height),
        fps: 0,
        sps,
        pps,
    })
}

fn first_nal(cur: &mut Cursor, count: u8) -> Option<Vec<u8>> {
    let mut first = Vec::new();
    for index in 0..count {
        let len = usize::from(cur.u16()?);
        let nal = cur.take(len)?;
        if index == 0 {
            first = nal.to_vec();
        }
    }
    Some(first)
}

fn parse_mp4a(data: &[u8], entry: &BoxRef, mdhd: &BoxRef) -> Result<AudioFormat> {
    let short = || invalid("short mp4a");
    let mut head = Cursor::new(entry.payload_bytes(data));
    head.take(16).ok_or_else(short)?;
    let channels = head.u16().ok_or_else(short)?;

    let mut mdhd_cur = Cursor::new(mdhd.payload_bytes(data));
    let version = mdhd_cur.u8().ok_or_else(short)?;
    mdhd_cur.take(3).ok_or_else(short)?;
    mdhd_cur
        .take(if version == 1 { 16 } else { 8 })
        .ok_or_else(short)?;
    let sample_rate = mdhd_cur.u32().ok_or_else(short)?;

    let esds = Boxes::within(
        data,
        entry.payload.start + MP4A_HEADER_LEN..entry.payload.end,
    )
    .find(|b| &b.fourcc == b"esds")
    .ok_or_else(|| invalid("no esds"))?;
    let (bitrate, asc) = parse_esds(esds.payload_bytes(data)).ok_or_else(|| invalid("bad esds"))?;
    Ok(AudioFormat {
        sample_rate,
        channels,
        bitrate,
        asc,
    })
}

fn descriptor(bytes: &[u8]) -> Option<(u8, &[u8])> {
    let mut cur = Cursor::new(bytes);
    let tag = cur.u8()?;
    let mut len = 0usize;
    for _ in 0..4 {
        let byte = cur.u8()?;
        len = (len << 7) | usize::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            break;
        }
    }
    Some((tag, cur.take(len)?))
}

fn parse_esds(payload: &[u8]) -> Option<(u32, Vec<u8>)> {
    let mut cur = Cursor::new(payload);
    cur.take(4)?;
    let (_, es) = descriptor(cur.rest())?;
    let (_, config) = descriptor(es.get(3..)?)?;
    let mut fields = Cursor::new(config);
    fields.take(5 + 4)?;
    let bitrate = fields.u32()?;
    let (tag, asc) = descriptor(fields.rest())?;
    (tag == 5).then(|| (bitrate, asc.to_vec()))
}
