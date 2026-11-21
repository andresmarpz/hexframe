use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::finalize::finalize;
use crate::{Mp4Error, Result};

pub(crate) mod cursor;
pub(crate) mod formats;
pub(crate) mod fragments;

use formats::read_formats;
use fragments::{Run, build_tables, first_video_duration, parse_moof};

const PLACEHOLDER_LEN: u64 = 16;
const FINALIZED_FREE_LEN: u64 = 8;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Header {
    pub(crate) fourcc: [u8; 4],
    pub(crate) size: u64,
}

pub(crate) struct Scan {
    pub(crate) placeholder_offset: u64,
    pub(crate) init_moov: Vec<u8>,
    pub(crate) runs: Vec<Run>,
    pub(crate) fragments: u32,
    pub(crate) good_end: u64,
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

pub(crate) fn already_finalized(part: &Path) -> Result<bool> {
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
    let (video, audio) = read_formats(&scan.init_moov, first_video_duration(&scan.runs))?;
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

pub(crate) fn header_at(file: &mut File, pos: u64, len: u64) -> Result<Option<Header>> {
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

pub(crate) fn read_exact_at(file: &mut File, pos: u64, len: u64) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(pos))?;
    let mut buf =
        vec![0u8; usize::try_from(len).map_err(|_| Mp4Error::Invalid("box too large".into()))?];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

pub(crate) fn scan(file: &mut File) -> Result<Scan> {
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
    let mut fragments = 0u32;
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
        fragments += 1;
        pos = mdat_pos + mdat.size;
        good_end = pos;
    }
    Ok(Scan {
        placeholder_offset,
        init_moov,
        runs,
        fragments,
        good_end,
    })
}
