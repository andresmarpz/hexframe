use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};

use crate::fragmented::BRANDS;
use crate::moov::{Chunk, Chunking, SampleRow, SampleTables, write_ftyp, write_full_moov};
use crate::{Mp4Error, Result};

const MDAT_SMALL_HEADER: u64 = 8;
const MDAT_LARGE_HEADER: u64 = 16;

pub(crate) struct Slot {
    pub track: Track,
    pub dts: i64,
    pub duration: u32,
    pub key: bool,
    pub len: usize,
}

pub(crate) struct Layout {
    pub kept: Vec<usize>,
    pub tables: SampleTables,
    pub chunking: Chunking,
    pub payload_len: u64,
}

pub(crate) type Fill<'a> = dyn FnMut(usize, &mut dyn Write) -> Result<()> + 'a;

pub fn write_clip(
    path: &Path,
    video: &VideoFormat,
    audio: Option<&AudioFormat>,
    packets: &[Arc<Packet>],
) -> Result<()> {
    let slots: Vec<Slot> = packets
        .iter()
        .map(|p| Slot {
            track: p.track,
            dts: p.dts,
            duration: p.duration,
            key: p.key,
            len: p.data.len(),
        })
        .collect();
    let layout = plan(audio, &slots)?;
    write_laid_out(path, video, audio, layout, &mut |index, out| {
        out.write_all(&packets[index].data).map_err(Mp4Error::from)
    })
}

pub(crate) fn write_laid_out(
    path: &Path,
    video: &VideoFormat,
    audio: Option<&AudioFormat>,
    mut layout: Layout,
    fill: &mut Fill<'_>,
) -> Result<()> {
    let ftyp = write_ftyp(b"isom", &BRANDS);
    let mdat_header_len = if layout.payload_len + MDAT_SMALL_HEADER > u64::from(u32::MAX) {
        MDAT_LARGE_HEADER
    } else {
        MDAT_SMALL_HEADER
    };

    let moov_len = write_full_moov(video, audio, &layout.tables, &layout.chunking).len() as u64;
    shift_offsets(&mut layout, ftyp.len() as u64 + moov_len + mdat_header_len);
    let moov = write_full_moov(video, audio, &layout.tables, &layout.chunking);
    if moov.len() as u64 != moov_len {
        return Err(Mp4Error::Invalid("moov size changed with offsets".into()));
    }

    let tmp = tmp_path(path);
    let result = write_file(&tmp, &ftyp, &moov, mdat_header_len, &layout, fill)
        .and_then(|()| std::fs::rename(&tmp, path).map_err(Mp4Error::from));
    if result.is_err() {
        std::fs::remove_file(&tmp).ok();
    }
    result
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

fn write_file(
    tmp: &Path,
    ftyp: &[u8],
    moov: &[u8],
    mdat_header_len: u64,
    layout: &Layout,
    fill: &mut Fill<'_>,
) -> Result<()> {
    let mut out = BufWriter::new(File::create(tmp)?);
    out.write_all(ftyp)?;
    out.write_all(moov)?;
    let mdat_size = layout.payload_len + mdat_header_len;
    if mdat_header_len == MDAT_LARGE_HEADER {
        out.write_all(&1u32.to_be_bytes())?;
        out.write_all(b"mdat")?;
        out.write_all(&mdat_size.to_be_bytes())?;
    } else {
        out.write_all(&(mdat_size as u32).to_be_bytes())?;
        out.write_all(b"mdat")?;
    }
    for &index in &layout.kept {
        fill(index, &mut out)?;
    }
    out.flush()?;
    out.get_ref().sync_data()?;
    Ok(())
}

fn shift_offsets(layout: &mut Layout, delta: u64) {
    let rows = layout
        .tables
        .video
        .iter_mut()
        .chain(layout.tables.audio.iter_mut());
    for row in rows {
        row.offset += delta;
    }
    let chunks = layout
        .chunking
        .video
        .iter_mut()
        .chain(layout.chunking.audio.iter_mut());
    for chunk in chunks {
        chunk.offset += delta;
    }
}

pub(crate) fn plan(audio: Option<&AudioFormat>, slots: &[Slot]) -> Result<Layout> {
    let idr = slots
        .iter()
        .find(|s| s.track == Track::Video)
        .filter(|s| s.key)
        .ok_or_else(|| Mp4Error::Invalid("clip does not start with a video key frame".into()))?;

    let mut kept: Vec<usize> = Vec::with_capacity(slots.len());
    for (index, slot) in slots.iter().enumerate() {
        match (slot.track, audio) {
            (Track::Video, _) => kept.push(index),
            (Track::Audio, None) => {
                return Err(Mp4Error::Invalid(
                    "audio packet without audio format".into(),
                ));
            }
            (Track::Audio, Some(format)) => {
                if !ends_before(slot.dts, slot.duration, format.sample_rate, idr.dts) {
                    kept.push(index);
                }
            }
        }
    }

    let video_durations = durations(slots, &kept, Track::Video)?;
    let audio_durations = durations(slots, &kept, Track::Audio)?;
    let mut video_durations = video_durations.into_iter();
    let mut audio_durations = audio_durations.into_iter();

    let mut tables = SampleTables::default();
    let mut chunking = Chunking::default();
    let mut position = 0u64;
    let mut last_track = None;
    for &index in &kept {
        let slot = &slots[index];
        let (rows, chunks, duration) = match slot.track {
            Track::Video => (
                &mut tables.video,
                &mut chunking.video,
                video_durations.next(),
            ),
            Track::Audio => (
                &mut tables.audio,
                &mut chunking.audio,
                audio_durations.next(),
            ),
        };
        if last_track != Some(slot.track) {
            chunks.push(Chunk {
                first_sample: rows.len() as u32,
                count: 0,
                offset: position,
            });
            last_track = Some(slot.track);
        }
        if let Some(chunk) = chunks.last_mut() {
            chunk.count += 1;
        }
        rows.push(SampleRow {
            offset: position,
            size: u32::try_from(slot.len)
                .map_err(|_| Mp4Error::Invalid("sample larger than 4 GiB".into()))?,
            duration: duration.unwrap_or(0),
            sync: slot.key,
        });
        position += slot.len as u64;
    }

    Ok(Layout {
        kept,
        tables,
        chunking,
        payload_len: position,
    })
}

pub(crate) fn ends_before(dts: i64, duration: u32, sample_rate: u32, idr_dts: i64) -> bool {
    let end = dts + i64::from(duration);
    i128::from(end) * i128::from(VideoFormat::TIMESCALE)
        <= i128::from(idr_dts) * i128::from(sample_rate)
}

fn durations(slots: &[Slot], kept: &[usize], track: Track) -> Result<Vec<u32>> {
    let of_track: Vec<&Slot> = kept
        .iter()
        .map(|&i| &slots[i])
        .filter(|s| s.track == track)
        .collect();
    let mut out = Vec::with_capacity(of_track.len());
    for pair in of_track.windows(2) {
        let delta = pair[1].dts - pair[0].dts;
        if delta <= 0 {
            return Err(Mp4Error::Invalid(format!(
                "dts {} does not follow {}",
                pair[1].dts, pair[0].dts
            )));
        }
        out.push(u32::try_from(delta).unwrap_or(u32::MAX));
    }
    if let Some(last) = of_track.last() {
        let fallback = out.last().copied().unwrap_or(0);
        out.push(if last.duration > 0 {
            last.duration
        } else {
            fallback
        });
    }
    Ok(out)
}
