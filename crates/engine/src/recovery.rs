use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use greyscreen_core::library;
use greyscreen_mp4::reader::find_box;
use greyscreen_mp4::recover::recover;

use crate::writer::{PART_FILE, VIDEO_FILE};

const FREE_AND_MDAT_LEN: usize = 16;
const TIMESCALE_V0: usize = 12;

pub(crate) fn find_parts(output_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(output_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path().join(PART_FILE))
        .filter(|part| part.is_file())
        .collect()
}

pub(crate) fn recover_part(part: &Path) {
    let Some(folder) = part.parent() else {
        return;
    };
    let finished = match finish(part) {
        Ok(finished) => finished,
        Err(err) => {
            tracing::error!(part = %part.display(), %err, "cannot recover interrupted recording");
            return;
        }
    };
    tracing::info!(file = %finished.display(), "recovered interrupted recording");
    update_sidecar(folder, &finished);
}

fn finish(part: &Path) -> Result<PathBuf, String> {
    let finished = part.with_file_name(VIDEO_FILE);
    if already_finalized(part).map_err(|e| e.to_string())? {
        fs::rename(part, &finished).map_err(|e| e.to_string())?;
        return Ok(finished);
    }
    recover(part).map_err(|e| e.to_string())
}

fn read_header(file: &mut File, pos: u64) -> std::io::Result<[u8; 8]> {
    let mut head = [0u8; 8];
    file.seek(SeekFrom::Start(pos))?;
    file.read_exact(&mut head)?;
    Ok(head)
}

fn already_finalized(part: &Path) -> std::io::Result<bool> {
    let mut file = File::open(part)?;
    let Ok(ftyp) = read_header(&mut file, 0) else {
        return Ok(false);
    };
    if &ftyp[4..8] != b"ftyp" {
        return Ok(false);
    }
    let ftyp_size = u64::from(u32::from_be_bytes([ftyp[0], ftyp[1], ftyp[2], ftyp[3]]));
    let mut tail = [0u8; FREE_AND_MDAT_LEN];
    file.seek(SeekFrom::Start(ftyp_size))?;
    if file.read_exact(&mut tail).is_err() {
        return Ok(false);
    }
    let small =
        tail[0..4] == 8u32.to_be_bytes() && &tail[4..8] == b"free" && &tail[12..16] == b"mdat";
    let large = tail[0..4] == 1u32.to_be_bytes() && &tail[4..8] == b"mdat";
    Ok(small || large)
}

fn update_sidecar(folder: &Path, video: &Path) {
    let Ok(mut meta) = library::read_meta(folder) else {
        return;
    };
    if meta.duration_secs > 0.0 {
        return;
    }
    let Some(duration) = movie_duration_secs(video) else {
        return;
    };
    meta.duration_secs = duration;
    if meta.video.is_empty() {
        meta.video = VIDEO_FILE.to_owned();
    }
    if let Err(err) = library::write_meta(folder, &meta) {
        tracing::warn!(folder = %folder.display(), %err, "cannot update recovered sidecar");
    }
}

fn movie_duration_secs(video: &Path) -> Option<f64> {
    let mut file = File::open(video).ok()?;
    let len = file.metadata().ok()?.len();
    let mut pos = 0u64;
    while pos + 8 <= len {
        let head = read_header(&mut file, pos).ok()?;
        let mut size = u64::from(u32::from_be_bytes([head[0], head[1], head[2], head[3]]));
        let mut header_len = 8;
        if size == 1 {
            let mut large = [0u8; 8];
            file.read_exact(&mut large).ok()?;
            size = u64::from_be_bytes(large);
            header_len = 16;
        } else if size == 0 {
            size = len - pos;
        }
        if size < header_len {
            return None;
        }
        if &head[4..8] == b"moov" {
            let mut moov = vec![0u8; usize::try_from(size).ok()?];
            file.seek(SeekFrom::Start(pos)).ok()?;
            file.read_exact(&mut moov).ok()?;
            return mvhd_duration(&moov);
        }
        pos += size;
    }
    None
}

fn mvhd_duration(moov: &[u8]) -> Option<f64> {
    let mvhd = find_box(moov, b"moov")?.find(moov, b"mvhd")?;
    let body = mvhd.payload_bytes(moov);
    let be32 = |at: usize| Some(u32::from_be_bytes(body.get(at..at + 4)?.try_into().ok()?));
    let (timescale, duration) = if *body.first()? == 1 {
        let at = 20;
        let wide = u64::from_be_bytes(body.get(at + 4..at + 12)?.try_into().ok()?);
        (be32(at)?, wide)
    } else {
        (be32(TIMESCALE_V0)?, u64::from(be32(TIMESCALE_V0 + 4)?))
    };
    (timescale > 0).then(|| duration as f64 / f64::from(timescale))
}
