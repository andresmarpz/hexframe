use crate::moov::{AudioStart, SampleRow};
use crate::reader::{BoxRef, boxes};
use crate::recover::cursor::Cursor;
use crate::{Mp4Error, Result};

pub(super) struct Track {
    pub(super) track_id: u32,
    pub(super) rows: Vec<SampleRow>,
    pub(super) start: AudioStart,
}

pub(super) struct Tracks(Vec<Track>);

impl Tracks {
    pub(super) fn start(&self, track_id: u32) -> AudioStart {
        self.0
            .iter()
            .find(|t| t.track_id == track_id)
            .map_or_else(AudioStart::default, |t| t.start)
    }

    pub(super) fn rows(&self, track_id: u32) -> Vec<SampleRow> {
        self.0
            .iter()
            .find(|t| t.track_id == track_id)
            .map(|t| t.rows.clone())
            .unwrap_or_default()
    }
}

fn invalid(what: &str) -> Mp4Error {
    Mp4Error::Invalid(format!("moov: {what}"))
}

fn path(data: &[u8], start: BoxRef, names: &[&[u8; 4]]) -> Option<BoxRef> {
    names
        .iter()
        .try_fold(start, |parent, fourcc| parent.find(data, fourcc))
}

pub(super) fn parse_tracks(moov: &[u8]) -> Result<Tracks> {
    let root = boxes(moov)
        .find(|b| &b.fourcc == b"moov")
        .ok_or_else(|| invalid("missing"))?;
    let mut tracks = Vec::new();
    for trak in root.children(moov).filter(|b| &b.fourcc == b"trak") {
        let tkhd = trak.find(moov, b"tkhd").ok_or_else(|| invalid("no tkhd"))?;
        let track_id = track_id(tkhd.payload_bytes(moov)).ok_or_else(|| invalid("short tkhd"))?;
        let start = trak
            .find(moov, b"edts")
            .and_then(|edts| edts.find(moov, b"elst"))
            .and_then(|elst| edit_start(elst.payload_bytes(moov)))
            .unwrap_or_default();
        let stbl =
            path(moov, trak, &[b"mdia", b"minf", b"stbl"]).ok_or_else(|| invalid("no stbl"))?;
        tracks.push(Track {
            track_id,
            rows: rows(moov, &stbl)?,
            start,
        });
    }
    Ok(Tracks(tracks))
}

pub(crate) fn edit_start(payload: &[u8]) -> Option<AudioStart> {
    let mut cur = Cursor::new(payload);
    if cur.u8()? != 0 {
        return None;
    }
    cur.take(3)?;
    let count = cur.u32()?;
    let mut start = AudioStart::default();
    for _ in 0..count {
        let duration = cur.u32()?;
        let media_time = cur.u32()? as i32;
        cur.u32()?;
        if media_time < 0 {
            start.delay_ms += u64::from(duration);
        } else {
            start.skip_samples = media_time.unsigned_abs();
            break;
        }
    }
    Some(start)
}

fn track_id(payload: &[u8]) -> Option<u32> {
    let mut cur = Cursor::new(payload);
    let version = cur.u8()?;
    cur.take(3)?;
    cur.take(if version == 1 { 16 } else { 8 })?;
    cur.u32()
}

fn rows(data: &[u8], stbl: &BoxRef) -> Result<Vec<SampleRow>> {
    let sizes = stbl.find(data, b"stsz").ok_or_else(|| invalid("no stsz"))?;
    let sizes = sample_sizes(sizes.payload_bytes(data)).ok_or_else(|| invalid("bad stsz"))?;
    let stts = stbl.find(data, b"stts").ok_or_else(|| invalid("no stts"))?;
    let durations = sample_durations(stts.payload_bytes(data), sizes.len())
        .ok_or_else(|| invalid("bad stts"))?;
    let sync = match stbl.find(data, b"stss") {
        Some(stss) => {
            Some(sync_samples(stss.payload_bytes(data)).ok_or_else(|| invalid("bad stss"))?)
        }
        None => None,
    };
    let mut next_sync = sync.as_ref().map(|list| list.iter().peekable());
    Ok(sizes
        .into_iter()
        .zip(durations)
        .enumerate()
        .map(|(index, (size, duration))| {
            let sync = match next_sync.as_mut() {
                None => true,
                Some(list) => {
                    let hit = list.peek().is_some_and(|&&n| n as usize == index + 1);
                    if hit {
                        list.next();
                    }
                    hit
                }
            };
            SampleRow {
                offset: 0,
                size,
                duration,
                sync,
            }
        })
        .collect())
}

fn sample_sizes(payload: &[u8]) -> Option<Vec<u32>> {
    let mut cur = Cursor::new(payload);
    cur.take(4)?;
    let fixed = cur.u32()?;
    let count = cur.u32()? as usize;
    if fixed != 0 {
        return Some(vec![fixed; count]);
    }
    if cur.rest().len() / 4 < count {
        return None;
    }
    (0..count).map(|_| cur.u32()).collect()
}

fn sample_durations(payload: &[u8], samples: usize) -> Option<Vec<u32>> {
    let mut cur = Cursor::new(payload);
    cur.take(4)?;
    let entries = cur.u32()? as usize;
    let mut out = Vec::with_capacity(samples);
    for _ in 0..entries {
        let count = cur.u32()? as usize;
        let delta = cur.u32()?;
        if out.len() + count > samples {
            return None;
        }
        out.resize(out.len() + count, delta);
    }
    (out.len() == samples).then_some(out)
}

fn sync_samples(payload: &[u8]) -> Option<Vec<u32>> {
    let mut cur = Cursor::new(payload);
    cur.take(4)?;
    let count = cur.u32()? as usize;
    if cur.rest().len() / 4 < count {
        return None;
    }
    (0..count).map(|_| cur.u32()).collect()
}
