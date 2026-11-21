use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use greyscreen_core::library;
use greyscreen_mp4::reader::find_box;
use greyscreen_mp4::recover::recover;

use crate::api::{NoticeAction, NoticeLevel, UiMsg};
use crate::storage::ActiveFolder;
use crate::writer::{PART_FILE, VIDEO_FILE};

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

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Recovery {
    pub(crate) recovered: usize,
    pub(crate) failed: Vec<PathBuf>,
}

impl Recovery {
    pub(crate) fn notice(&self, output_dir: &Path) -> Option<UiMsg> {
        match self.failed.as_slice() {
            [] if self.recovered == 0 => None,
            [] => Some(UiMsg::Notice {
                level: NoticeLevel::Info,
                text: recovered_text(self.recovered),
            }),
            [folder] => Some(UiMsg::NoticeWithAction {
                level: NoticeLevel::Warn,
                text: format!("Couldn't recover {}", folder_name(folder)),
                action: NoticeAction::Reveal(folder.clone()),
            }),
            many => Some(UiMsg::NoticeWithAction {
                level: NoticeLevel::Warn,
                text: format!("Couldn't recover {} recordings", many.len()),
                action: NoticeAction::Reveal(output_dir.to_path_buf()),
            }),
        }
    }
}

fn recovered_text(count: usize) -> String {
    if count == 1 {
        "Recovered 1 recording that was cut off.".to_owned()
    } else {
        format!("Recovered {count} recordings that were cut off.")
    }
}

fn folder_name(folder: &Path) -> String {
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn is_active(folder: &Path, active: &ActiveFolder) -> bool {
    let running = active.lock().map(|id| id.clone()).unwrap_or_default();
    running.is_some_and(|id| id == folder_name(folder))
}

pub(crate) fn recover_parts(parts: &[PathBuf], active: &ActiveFolder) -> Recovery {
    let mut outcome = Recovery::default();
    for part in parts {
        let Some(folder) = part.parent() else {
            continue;
        };
        if is_active(folder, active) {
            continue;
        }
        match recover_part(part) {
            Ok(()) => outcome.recovered += 1,
            Err(err) => {
                tracing::error!(part = %part.display(), %err, "cannot recover interrupted recording");
                outcome.failed.push(folder.to_path_buf());
            }
        }
    }
    outcome
}

pub(crate) fn recover_part(part: &Path) -> Result<(), String> {
    let folder = part.parent().ok_or("no folder")?;
    let finished = finish(part)?;
    tracing::info!(file = %finished.display(), "recovered interrupted recording");
    update_sidecar(folder, &finished)
}

fn finish(part: &Path) -> Result<PathBuf, String> {
    recover(part).map_err(|e| e.to_string())
}

fn read_header(file: &mut File, pos: u64) -> std::io::Result<[u8; 8]> {
    let mut head = [0u8; 8];
    file.seek(SeekFrom::Start(pos))?;
    file.read_exact(&mut head)?;
    Ok(head)
}

fn update_sidecar(folder: &Path, video: &Path) -> Result<(), String> {
    let mut meta =
        library::read_meta(folder).map_err(|err| format!("no recording details: {err}"))?;
    if meta.video.is_empty() {
        meta.video = VIDEO_FILE.to_owned();
    }
    if meta.duration_secs <= 0.0
        && let Some(duration) = movie_duration_secs(video)
    {
        meta.duration_secs = duration;
    }
    meta.recovered = true;
    library::write_meta(folder, &meta).map_err(|err| format!("cannot update details: {err}"))
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

#[cfg(test)]
mod tests;
