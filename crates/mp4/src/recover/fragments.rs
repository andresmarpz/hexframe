use std::ops::Range;

use super::cursor::Cursor;
use crate::moov::{
    AUDIO_TRACK_ID, Chunk, Chunking, NON_SYNC_SAMPLE_FLAGS, SYNC_SAMPLE_FLAGS, SampleRow,
    SampleTables, VIDEO_TRACK_ID,
};
use crate::reader::boxes;

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct Sample {
    offset: u64,
    size: u32,
    pub(crate) duration: u32,
    sync: bool,
}

#[derive(Debug)]
pub(crate) struct Run {
    pub(crate) track_id: u32,
    pub(crate) samples: Vec<Sample>,
}

pub(crate) fn parse_moof(
    data: &[u8],
    moof_start: u64,
    mdat_payload: &Range<u64>,
) -> Option<Vec<Run>> {
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

pub(crate) fn build_tables(runs: &[Run]) -> (SampleTables, Chunking) {
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

pub(crate) fn first_video_duration(runs: &[Run]) -> u32 {
    runs.iter()
        .find(|r| r.track_id == VIDEO_TRACK_ID)
        .and_then(|r| r.samples.first())
        .map_or(0, |s| s.duration)
}
