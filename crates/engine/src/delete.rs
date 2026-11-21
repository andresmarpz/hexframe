use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::Duration;

use greyscreen_core::library::META_FILE;
use thiserror::Error;

use crate::api::{NoticeLevel, UiMsg};
use crate::control::Sink;
use crate::library;

#[derive(Debug, Error)]
pub(crate) enum DeleteError {
    #[error("that folder is not a recording in the output folder")]
    NotARecording,
    #[error("the recording is still being written")]
    InProgress,
    #[error("the file is in use")]
    InUse,
    #[error("{0}")]
    Move(#[source] io::Error),
}

const IN_USE_ATTEMPTS: u32 = 10;
const IN_USE_DELAY: Duration = if cfg!(test) {
    Duration::from_millis(5)
} else {
    Duration::from_millis(250)
};

pub(crate) type Mover<'a> = &'a dyn Fn(&Path) -> io::Result<()>;

pub(crate) fn recording_folder(output_dir: &Path, id: &str) -> Result<PathBuf, DeleteError> {
    let mut parts = Path::new(id).components();
    let single = matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(_)), None)
    );
    if !single {
        return Err(DeleteError::NotARecording);
    }
    let folder = output_dir.join(id);
    let is_real_dir = fs::symlink_metadata(&folder).is_ok_and(|meta| meta.is_dir());
    if !is_real_dir || !folder.join(META_FILE).is_file() {
        return Err(DeleteError::NotARecording);
    }
    let root = fs::canonicalize(output_dir).map_err(|_| DeleteError::NotARecording)?;
    let resolved = fs::canonicalize(&folder).map_err(|_| DeleteError::NotARecording)?;
    if resolved.parent() != Some(root.as_path()) {
        return Err(DeleteError::NotARecording);
    }
    Ok(folder)
}

pub(crate) fn delete_recording(
    output_dir: &Path,
    id: &str,
    active: Option<&str>,
    mover: Mover,
) -> Result<(), DeleteError> {
    if active == Some(id) {
        return Err(DeleteError::InProgress);
    }
    let folder = recording_folder(output_dir, id)?;
    for attempt in 1..=IN_USE_ATTEMPTS {
        match mover(&folder) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == io::ErrorKind::ResourceBusy => {
                tracing::info!(attempt, id, "the recording is in use, trying again");
                if attempt < IN_USE_ATTEMPTS {
                    thread::sleep(IN_USE_DELAY);
                }
            }
            Err(err) => return Err(DeleteError::Move(err)),
        }
    }
    Err(DeleteError::InUse)
}

fn run(output_dir: &Path, id: &str, active: Option<&str>, mover: Mover, sink: &Sink) {
    match delete_recording(output_dir, id, active, mover) {
        Ok(()) => sink(UiMsg::Notice {
            level: NoticeLevel::Info,
            text: format!("Moved the recording to the {}", bin_name()),
        }),
        Err(err) => {
            tracing::warn!(%err, id, "cannot delete the recording");
            sink(UiMsg::Notice {
                level: NoticeLevel::Error,
                text: format!("Couldn't delete: {err}"),
            });
        }
    }
    sink(UiMsg::Library(library::entries(output_dir)));
}

pub(crate) fn spawn(output_dir: PathBuf, id: String, active: Option<String>, sink: Sink) {
    let spawned = thread::Builder::new()
        .name("recording-delete".into())
        .spawn(move || run(&output_dir, &id, active.as_deref(), &move_to_bin, &sink));
    if let Err(err) = spawned {
        tracing::error!(%err, "cannot start delete worker thread");
    }
}

#[cfg(windows)]
fn bin_name() -> &'static str {
    "Recycle Bin"
}

#[cfg(not(windows))]
fn bin_name() -> &'static str {
    "Trash"
}

#[cfg(windows)]
fn move_to_bin(folder: &Path) -> io::Result<()> {
    greyscreen_capture::recycle::move_to_recycle_bin(folder)
}

#[cfg(not(windows))]
fn move_to_bin(folder: &Path) -> io::Result<()> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home folder"))?;
    move_into(folder, &home.join(".Trash"))
}

#[cfg(any(not(windows), test))]
pub(crate) fn move_into(folder: &Path, bin: &Path) -> io::Result<()> {
    fs::create_dir_all(bin)?;
    let name = folder
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no folder name"))?;
    let mut target = bin.join(name);
    let mut copy = 1;
    while fs::symlink_metadata(&target).is_ok() {
        copy += 1;
        let mut numbered = name.to_os_string();
        numbered.push(format!(" {copy}"));
        target = bin.join(numbered);
    }
    fs::rename(folder, target)
}

#[cfg(test)]
mod tests;
