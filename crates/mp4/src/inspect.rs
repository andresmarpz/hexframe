use std::fs::File;
use std::path::Path;

use greyscreen_core::media::{AudioFormat, VideoFormat};

use crate::moov::{AUDIO_TRACK_ID, AudioStart, SampleRow, SampleTables, VIDEO_TRACK_ID};
use crate::recover::formats::read_formats;
use crate::recover::fragments::{build_tables, first_video_duration};
use crate::recover::{already_finalized, header_at, read_exact_at, scan};
use crate::{Mp4Error, Result};

mod tables;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Finalized,
    Unfinalized { fragments: u32, trailing_bytes: u64 },
}

#[derive(Debug, Clone)]
pub struct Inspected {
    pub layout: Layout,
    pub bytes: u64,
    pub video: VideoFormat,
    pub audio: Option<AudioFormat>,
    pub tables: SampleTables,
    pub audio_start: AudioStart,
}

pub fn inspect(path: &Path) -> Result<Inspected> {
    let mut file = File::open(path)?;
    let bytes = file.metadata()?.len();
    if already_finalized(path)? {
        let moov = read_trailing_moov(&mut file, bytes)?;
        let tracks = tables::parse_tracks(&moov)?;
        let video_rows = tracks.rows(VIDEO_TRACK_ID);
        let first = video_rows.first().map_or(0, |row| row.duration);
        let (video, audio) = read_formats(&moov, first)?;
        return Ok(Inspected {
            layout: Layout::Finalized,
            bytes,
            video,
            audio,
            audio_start: tracks.start(AUDIO_TRACK_ID),
            tables: SampleTables {
                video: video_rows,
                audio: tracks.rows(AUDIO_TRACK_ID),
            },
        });
    }
    let scanned = scan(&mut file)?;
    let (video, audio) = read_formats(&scanned.init_moov, first_video_duration(&scanned.runs))?;
    let (tables, _) = build_tables(&scanned.runs);
    Ok(Inspected {
        layout: Layout::Unfinalized {
            fragments: scanned.fragments,
            trailing_bytes: bytes.saturating_sub(scanned.good_end),
        },
        bytes,
        video,
        audio,
        tables,
        audio_start: scanned.audio_start,
    })
}

fn read_trailing_moov(file: &mut File, len: u64) -> Result<Vec<u8>> {
    let mut pos = 0;
    while let Some(header) = header_at(file, pos, len)? {
        if &header.fourcc == b"moov" {
            if pos + header.size > len {
                return Err(Mp4Error::Invalid("moov box is cut off".into()));
            }
            return read_exact_at(file, pos, header.size);
        }
        pos += header.size.max(8);
    }
    Err(Mp4Error::Invalid("no moov box in the file".into()))
}

pub fn sample_secs(rows: &[SampleRow], timescale: u32) -> f64 {
    rows.iter().map(|r| u64::from(r.duration)).sum::<u64>() as f64 / f64::from(timescale.max(1))
}

#[cfg(test)]
mod tests;
