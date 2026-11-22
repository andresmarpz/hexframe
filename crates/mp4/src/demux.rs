use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use greyscreen_core::media::{AudioFormat, VideoFormat};

use crate::inspect::tables::edit_start;
use crate::reader::{BoxRef, boxes};
use crate::{Mp4Error, Result};

const MAX_MOOV_BYTES: u64 = 256 * 1024 * 1024;
const AVC1_HEADER_LEN: usize = 78;
const MP4A_HEADER_LEN: usize = 28;
const DEFAULT_AUDIO_BITRATE: u32 = 192_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRef {
    pub offset: u64,
    pub size: u32,
    pub dts: i64,
    pub duration: u32,
    pub key: bool,
}

impl SampleRef {
    pub fn end(&self) -> i64 {
        self.dts + i64::from(self.duration)
    }
}

#[derive(Debug, Clone)]
pub struct Movie {
    pub path: PathBuf,
    pub video: VideoFormat,
    pub audio: Option<AudioFormat>,
    pub video_samples: Vec<SampleRef>,
    pub audio_samples: Vec<SampleRef>,
}

impl Movie {
    pub fn open(path: &Path) -> Result<Movie> {
        let mut file = File::open(path)?;
        let moov = read_moov(&mut file)?;
        parse_moov(path, &moov)
    }

    pub fn video_secs(&self) -> f64 {
        let ticks: i64 = self.video_samples.last().map_or(0, SampleRef::end);
        ticks as f64 / f64::from(VideoFormat::TIMESCALE)
    }

    pub fn reader(&self) -> Result<SampleReader> {
        Ok(SampleReader {
            file: File::open(&self.path)?,
        })
    }
}

#[derive(Debug)]
pub struct SampleReader {
    file: File,
}

impl SampleReader {
    pub fn read(&mut self, sample: &SampleRef) -> Result<Vec<u8>> {
        self.file.seek(SeekFrom::Start(sample.offset))?;
        let mut data = vec![0u8; sample.size as usize];
        self.file.read_exact(&mut data)?;
        Ok(data)
    }
}

fn read_moov(file: &mut File) -> Result<Vec<u8>> {
    let len = file.metadata()?.len();
    let mut pos = 0u64;
    while pos + 8 <= len {
        file.seek(SeekFrom::Start(pos))?;
        let mut header = [0u8; 16];
        let got = file.read(&mut header)?;
        if got < 8 {
            break;
        }
        let size32 = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let (size, header_len) = match size32 {
            0 => (len - pos, 8),
            1 if got >= 16 => (
                u64::from_be_bytes([
                    header[8], header[9], header[10], header[11], header[12], header[13],
                    header[14], header[15],
                ]),
                16,
            ),
            1 => break,
            n => (u64::from(n), 8),
        };
        if size < header_len {
            break;
        }
        if &header[4..8] == b"moov" {
            if size > MAX_MOOV_BYTES || pos + size > len {
                return Err(Mp4Error::Invalid("moov box is damaged".into()));
            }
            file.seek(SeekFrom::Start(pos))?;
            let mut moov = vec![0u8; size as usize];
            file.read_exact(&mut moov)?;
            return Ok(moov);
        }
        pos += size;
    }
    Err(Mp4Error::Invalid("file has no moov box".into()))
}

fn invalid(what: &str) -> Mp4Error {
    Mp4Error::Invalid(format!("unreadable mp4: {what}"))
}

fn be16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn be32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn be64(data: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(data.get(at..at + 8)?.try_into().ok()?))
}

struct RawTrack {
    kind: [u8; 4],
    timescale: u32,
    entry: Vec<u8>,
    samples: Vec<SampleRef>,
}

fn parse_moov(path: &Path, moov: &[u8]) -> Result<Movie> {
    let root = boxes(moov)
        .find(|b| &b.fourcc == b"moov")
        .ok_or_else(|| invalid("no moov"))?;
    let mut video = None;
    let mut audio = None;
    for trak in root.children(moov).filter(|b| &b.fourcc == b"trak") {
        let raw = parse_trak(moov, &trak)?;
        match &raw.kind {
            b"vide" if video.is_none() => video = Some(raw),
            b"soun" if audio.is_none() => audio = Some(raw),
            _ => {}
        }
    }
    let video = video.ok_or_else(|| invalid("no video track"))?;
    if video.timescale != VideoFormat::TIMESCALE {
        return Err(invalid("video timescale is not 90000"));
    }
    let video_format = video_format(&video)?;
    let audio_format = audio.as_ref().map(audio_format).transpose()?;
    Ok(Movie {
        path: path.to_owned(),
        video: video_format,
        audio: audio_format,
        video_samples: video.samples,
        audio_samples: audio.map(|a| a.samples).unwrap_or_default(),
    })
}

fn parse_trak(data: &[u8], trak: &BoxRef) -> Result<RawTrack> {
    let mdia = trak.find(data, b"mdia").ok_or_else(|| invalid("no mdia"))?;
    let mdhd = mdia.find(data, b"mdhd").ok_or_else(|| invalid("no mdhd"))?;
    let mdhd = mdhd.payload_bytes(data);
    let timescale = match mdhd.first() {
        Some(1) => be32(mdhd, 20),
        _ => be32(mdhd, 12),
    }
    .ok_or_else(|| invalid("short mdhd"))?;
    let hdlr = mdia.find(data, b"hdlr").ok_or_else(|| invalid("no hdlr"))?;
    let kind: [u8; 4] = hdlr
        .payload_bytes(data)
        .get(8..12)
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| invalid("short hdlr"))?;
    let minf = mdia.find(data, b"minf").ok_or_else(|| invalid("no minf"))?;
    let stbl = minf.find(data, b"stbl").ok_or_else(|| invalid("no stbl"))?;
    let entry = sample_entry(data, &stbl)?;
    let mut samples = samples(data, &stbl)?;
    if &kind == b"soun" {
        let start = trak
            .find(data, b"edts")
            .and_then(|edts| edts.find(data, b"elst"))
            .and_then(|elst| edit_start(elst.payload_bytes(data)))
            .unwrap_or_default();
        let shift = i64::try_from(u128::from(start.delay_ms) * u128::from(timescale) / 1000)
            .unwrap_or(i64::MAX)
            .saturating_sub(i64::from(start.skip_samples));
        for sample in &mut samples {
            sample.dts += shift;
        }
    }
    Ok(RawTrack {
        kind,
        timescale,
        entry,
        samples,
    })
}

fn sample_entry(data: &[u8], stbl: &BoxRef) -> Result<Vec<u8>> {
    let stsd = stbl.find(data, b"stsd").ok_or_else(|| invalid("no stsd"))?;
    let payload = stsd.payload_bytes(data);
    let entry = payload.get(8..).ok_or_else(|| invalid("short stsd"))?;
    let size = be32(entry, 0).ok_or_else(|| invalid("short sample entry"))? as usize;
    entry
        .get(..size)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| invalid("sample entry overruns stsd"))
}

fn full_box<'a>(data: &'a [u8], stbl: &BoxRef, fourcc: &[u8; 4]) -> Option<&'a [u8]> {
    let found = stbl.find(data, fourcc)?;
    found.payload_bytes(data).get(4..)
}

fn samples(data: &[u8], stbl: &BoxRef) -> Result<Vec<SampleRef>> {
    let stts = full_box(data, stbl, b"stts").ok_or_else(|| invalid("no stts"))?;
    let stsz = full_box(data, stbl, b"stsz").ok_or_else(|| invalid("no stsz"))?;
    let stsc = full_box(data, stbl, b"stsc").ok_or_else(|| invalid("no stsc"))?;
    let offsets = chunk_offsets(data, stbl)?;
    let sizes = sample_sizes(stsz)?;
    let durations = sample_durations(stts, sizes.len())?;
    let keys = sync_samples(full_box(data, stbl, b"stss"), sizes.len());
    let positions = sample_offsets(stsc, &offsets, &sizes)?;

    let mut dts = 0i64;
    let mut out = Vec::with_capacity(sizes.len());
    for (i, ((&size, &duration), &offset)) in
        sizes.iter().zip(&durations).zip(&positions).enumerate()
    {
        out.push(SampleRef {
            offset,
            size,
            dts,
            duration,
            key: keys[i],
        });
        dts += i64::from(duration);
    }
    Ok(out)
}

fn chunk_offsets(data: &[u8], stbl: &BoxRef) -> Result<Vec<u64>> {
    if let Some(co64) = full_box(data, stbl, b"co64") {
        let count = be32(co64, 0).ok_or_else(|| invalid("short co64"))? as usize;
        return (0..count)
            .map(|i| be64(co64, 4 + i * 8).ok_or_else(|| invalid("short co64")))
            .collect();
    }
    let stco = full_box(data, stbl, b"stco").ok_or_else(|| invalid("no chunk offsets"))?;
    let count = be32(stco, 0).ok_or_else(|| invalid("short stco"))? as usize;
    (0..count)
        .map(|i| {
            be32(stco, 4 + i * 4)
                .map(u64::from)
                .ok_or_else(|| invalid("short stco"))
        })
        .collect()
}

fn sample_sizes(stsz: &[u8]) -> Result<Vec<u32>> {
    let constant = be32(stsz, 0).ok_or_else(|| invalid("short stsz"))?;
    let count = be32(stsz, 4).ok_or_else(|| invalid("short stsz"))? as usize;
    if constant != 0 {
        return Ok(vec![constant; count]);
    }
    (0..count)
        .map(|i| be32(stsz, 8 + i * 4).ok_or_else(|| invalid("short stsz")))
        .collect()
}

fn sample_durations(stts: &[u8], count: usize) -> Result<Vec<u32>> {
    let runs = be32(stts, 0).ok_or_else(|| invalid("short stts"))? as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..runs {
        let n = be32(stts, 4 + i * 8).ok_or_else(|| invalid("short stts"))? as usize;
        let delta = be32(stts, 8 + i * 8).ok_or_else(|| invalid("short stts"))?;
        out.extend(std::iter::repeat_n(
            delta,
            n.min(count.saturating_sub(out.len())),
        ));
    }
    if out.len() != count {
        return Err(invalid("stts and stsz disagree"));
    }
    Ok(out)
}

fn sync_samples(stss: Option<&[u8]>, count: usize) -> Vec<bool> {
    let Some(stss) = stss else {
        return vec![true; count];
    };
    let mut keys = vec![false; count];
    let entries = be32(stss, 0).unwrap_or(0) as usize;
    for i in 0..entries {
        let Some(number) = be32(stss, 4 + i * 4) else {
            break;
        };
        if let Some(slot) = (number as usize)
            .checked_sub(1)
            .and_then(|n| keys.get_mut(n))
        {
            *slot = true;
        }
    }
    keys
}

fn sample_offsets(stsc: &[u8], chunks: &[u64], sizes: &[u32]) -> Result<Vec<u64>> {
    let runs = be32(stsc, 0).ok_or_else(|| invalid("short stsc"))? as usize;
    let mut table = Vec::with_capacity(runs);
    for i in 0..runs {
        let first = be32(stsc, 4 + i * 12).ok_or_else(|| invalid("short stsc"))? as usize;
        let per = be32(stsc, 8 + i * 12).ok_or_else(|| invalid("short stsc"))? as usize;
        table.push((first, per));
    }
    let mut out = Vec::with_capacity(sizes.len());
    let mut next = 0usize;
    for (chunk, &chunk_offset) in chunks.iter().enumerate() {
        let per = table
            .iter()
            .rev()
            .find(|(first, _)| *first <= chunk + 1)
            .map_or(0, |(_, per)| *per);
        let mut offset = chunk_offset;
        for _ in 0..per {
            let Some(&size) = sizes.get(next) else {
                return Ok(out);
            };
            out.push(offset);
            offset += u64::from(size);
            next += 1;
        }
    }
    if out.len() != sizes.len() {
        return Err(invalid("chunk tables do not cover every sample"));
    }
    Ok(out)
}

fn video_format(track: &RawTrack) -> Result<VideoFormat> {
    let entry = &track.entry;
    let width = be16(entry, 8 + 24).ok_or_else(|| invalid("short avc1"))?;
    let height = be16(entry, 8 + 26).ok_or_else(|| invalid("short avc1"))?;
    let children = entry
        .get(8 + AVC1_HEADER_LEN..)
        .ok_or_else(|| invalid("short avc1"))?;
    let avcc = boxes(children)
        .find(|b| &b.fourcc == b"avcC")
        .ok_or_else(|| invalid("no avcC"))?;
    let (sps, pps) = parse_avcc(avcc.payload_bytes(children))?;
    let frame = track
        .samples
        .iter()
        .map(|s| s.duration)
        .find(|d| *d > 0)
        .unwrap_or(1500);
    Ok(VideoFormat {
        width: u32::from(width),
        height: u32::from(height),
        fps: (f64::from(VideoFormat::TIMESCALE) / f64::from(frame)).round() as u32,
        sps,
        pps,
    })
}

fn parse_avcc(avcc: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let sps_len = be16(avcc, 6).ok_or_else(|| invalid("short avcC"))? as usize;
    let sps = avcc
        .get(8..8 + sps_len)
        .ok_or_else(|| invalid("short avcC"))?
        .to_vec();
    let after = 8 + sps_len;
    let pps_len = be16(avcc, after + 1).ok_or_else(|| invalid("short avcC"))? as usize;
    let pps = avcc
        .get(after + 3..after + 3 + pps_len)
        .ok_or_else(|| invalid("short avcC"))?
        .to_vec();
    Ok((sps, pps))
}

fn audio_format(track: &RawTrack) -> Result<AudioFormat> {
    let entry = &track.entry;
    let channels = be16(entry, 8 + 16).ok_or_else(|| invalid("short mp4a"))?;
    let children = entry
        .get(8 + MP4A_HEADER_LEN..)
        .ok_or_else(|| invalid("short mp4a"))?;
    let esds = boxes(children)
        .find(|b| &b.fourcc == b"esds")
        .ok_or_else(|| invalid("no esds"))?;
    let body = esds
        .payload_bytes(children)
        .get(4..)
        .ok_or_else(|| invalid("short esds"))?;
    let (asc, bitrate) = parse_esds(body)?;
    Ok(AudioFormat {
        sample_rate: track.timescale,
        channels,
        bitrate,
        asc,
    })
}

fn descriptor(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *data.first()?;
    let mut len = 0usize;
    let mut pos = 1;
    loop {
        let byte = *data.get(pos)?;
        pos += 1;
        len = (len << 7) | usize::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            break;
        }
    }
    let body = data.get(pos..pos + len)?;
    Some((tag, body, data.get(pos + len..)?))
}

fn parse_esds(data: &[u8]) -> Result<(Vec<u8>, u32)> {
    let short = || invalid("short esds");
    let (tag, es, _) = descriptor(data).ok_or_else(short)?;
    if tag != 0x03 {
        return Err(short());
    }
    let (tag, config, _) = descriptor(es.get(3..).ok_or_else(short)?).ok_or_else(short)?;
    if tag != 0x04 {
        return Err(short());
    }
    let bitrate = be32(config, 9)
        .filter(|b| *b > 0)
        .unwrap_or(DEFAULT_AUDIO_BITRATE);
    let (tag, asc, _) = descriptor(config.get(13..).ok_or_else(short)?).ok_or_else(short)?;
    if tag != 0x05 {
        return Err(short());
    }
    Ok((asc.to_vec(), bitrate))
}
