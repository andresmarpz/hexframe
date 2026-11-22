use std::io::SeekFrom;

use greyscreen_core::media::{AudioFormat, VideoFormat};

use crate::moov::{Chunking, SampleRow, SampleTables, write_full_moov_delayed};
use crate::sink::Sink;
use crate::{Mp4Error, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinishedInfo {
    pub duration_secs: f64,
    pub bytes: u64,
}

pub fn placeholder_bytes(placeholder_offset: u64, moov_offset: u64) -> Result<[u8; 16]> {
    let mdat_start = placeholder_offset + 8;
    let mdat_size = moov_offset
        .checked_sub(mdat_start)
        .ok_or_else(|| Mp4Error::Invalid("moov starts before the placeholder".into()))?;
    let mut out = [0u8; 16];
    match u32::try_from(mdat_size) {
        Ok(size) => {
            out[0..4].copy_from_slice(&8u32.to_be_bytes());
            out[4..8].copy_from_slice(b"free");
            out[8..12].copy_from_slice(&size.to_be_bytes());
            out[12..16].copy_from_slice(b"mdat");
        }
        Err(_) => {
            out[0..4].copy_from_slice(&1u32.to_be_bytes());
            out[4..8].copy_from_slice(b"mdat");
            out[8..16].copy_from_slice(&(mdat_size + 8).to_be_bytes());
        }
    }
    Ok(out)
}

fn ticks(rows: &[SampleRow]) -> u64 {
    rows.iter().map(|r| u64::from(r.duration)).sum()
}

pub fn duration_secs(
    audio: Option<&AudioFormat>,
    tables: &SampleTables,
    audio_delay_ms: u64,
) -> f64 {
    let video_secs = ticks(&tables.video) as f64 / f64::from(VideoFormat::TIMESCALE);
    let audio_secs = audio.map_or(0.0, |a| {
        ticks(&tables.audio) as f64 / f64::from(a.sample_rate) + audio_delay_ms as f64 / 1000.0
    });
    video_secs.max(audio_secs)
}

pub fn finalize(
    file: &mut impl Sink,
    placeholder_offset: u64,
    video: &VideoFormat,
    audio: Option<&AudioFormat>,
    tables: &SampleTables,
    chunking: &Chunking,
    audio_delay_ms: u64,
) -> Result<FinishedInfo> {
    let moov = write_full_moov_delayed(video, audio, tables, chunking, audio_delay_ms);
    let moov_offset = file.seek(SeekFrom::End(0))?;
    let placeholder = placeholder_bytes(placeholder_offset, moov_offset)?;
    file.write_all(&moov)?;
    file.sync()?;
    file.seek(SeekFrom::Start(placeholder_offset))?;
    file.write_all(&placeholder)?;
    file.sync()?;
    Ok(FinishedInfo {
        duration_secs: duration_secs(audio, tables, audio_delay_ms),
        bytes: moov_offset + moov.len() as u64,
    })
}
