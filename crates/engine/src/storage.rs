use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use greyscreen_core::library::{self as core_library, RecordingMeta};
use greyscreen_core::retention::{Deletion, GameUsage, plan, plan_free_space};

use crate::api::{NoticeLevel, UiMsg};
use crate::control::Sink;
use crate::delete::recording_folder;
use crate::library;
use crate::writer::VIDEO_FILE;

pub(crate) type ActiveFolder = Arc<Mutex<Option<String>>>;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Cleanup {
    pub videos: usize,
    pub games: usize,
}

impl Cleanup {
    fn is_empty(&self) -> bool {
        self.videos == 0 && self.games == 0
    }

    fn notice(&self) -> String {
        let plural = |n: usize, one: &str, many: &str| {
            if n == 1 {
                format!("1 {one}")
            } else {
                format!("{n} {many}")
            }
        };
        let videos = plural(self.videos, "old game video", "old game videos");
        let games = plural(self.games, "whole game", "whole games");
        match (self.videos, self.games) {
            (_, 0) => format!("Storage limit reached: deleted {videos}."),
            (0, _) => format!("Storage limit reached: deleted {games}."),
            _ => format!("Storage limit reached: deleted {videos} and {games}."),
        }
    }
}

impl Cleanup {
    pub(crate) fn low_disk_notice(&self) -> String {
        let videos = if self.videos == 1 {
            "1 old game video".to_owned()
        } else {
            format!("{} old game videos", self.videos)
        };
        format!("Low disk space: deleted {videos} to make room.")
    }
}

fn active_id(active: &ActiveFolder) -> Option<String> {
    active.lock().map(|id| id.clone()).unwrap_or_default()
}

fn folder_bytes(folder: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(folder) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| Some((entry.path(), fs::symlink_metadata(entry.path()).ok()?)))
        .map(|(path, meta)| {
            if meta.is_dir() {
                folder_bytes(&path)
            } else if meta.is_file() {
                meta.len()
            } else {
                0
            }
        })
        .sum()
}

fn usage(output_dir: &Path, meta: &RecordingMeta) -> Option<GameUsage> {
    let folder = recording_folder(output_dir, &meta.id).ok()?;
    let video_bytes = fs::symlink_metadata(folder.join(VIDEO_FILE))
        .ok()
        .filter(|m| m.is_file())
        .map_or(0, |m| m.len());
    Some(GameUsage {
        id: meta.id.clone(),
        started_at: meta.started_at.timestamp(),
        video_bytes,
        other_bytes: folder_bytes(&folder).saturating_sub(video_bytes),
    })
}

fn delete_video(folder: &Path) -> io::Result<()> {
    match fs::remove_file(folder.join(VIDEO_FILE)) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    match core_library::read_meta(folder) {
        Ok(mut meta) => {
            meta.video = String::new();
            core_library::write_meta(folder, &meta).map_err(io::Error::other)
        }
        Err(err) => Err(io::Error::other(err)),
    }
}

fn usages(output_dir: &Path) -> Vec<GameUsage> {
    core_library::scan(output_dir)
        .iter()
        .filter_map(|meta| usage(output_dir, meta))
        .collect()
}

fn apply(output_dir: &Path, deletion: &Deletion, active: &ActiveFolder, cleanup: &mut Cleanup) {
    if active_id(active).as_deref() == Some(deletion.id()) {
        return;
    }
    let Ok(folder) = recording_folder(output_dir, deletion.id()) else {
        return;
    };
    match deletion {
        Deletion::Video { id } => match delete_video(&folder) {
            Ok(()) => cleanup.videos += 1,
            Err(err) => tracing::warn!(%err, id, "cannot delete the game video"),
        },
        Deletion::Game { id } => match fs::remove_dir_all(&folder) {
            Ok(()) => cleanup.games += 1,
            Err(err) => tracing::warn!(%err, id, "cannot delete the game folder"),
        },
    }
}

pub(crate) fn enforce(output_dir: &Path, limit_bytes: u64, active: &ActiveFolder) -> Cleanup {
    let games = usages(output_dir);
    let protected = active_id(active);
    let mut cleanup = Cleanup::default();
    for deletion in plan(&games, limit_bytes, protected.as_deref()) {
        apply(output_dir, &deletion, active, &mut cleanup);
    }
    cleanup
}

pub(crate) fn free_up(output_dir: &Path, needed_bytes: u64, active: &ActiveFolder) -> Cleanup {
    let games = usages(output_dir);
    let protected = active_id(active);
    let mut cleanup = Cleanup::default();
    for deletion in plan_free_space(&games, needed_bytes, protected.as_deref()) {
        apply(output_dir, &deletion, active, &mut cleanup);
    }
    cleanup
}

pub(crate) fn run(output_dir: &Path, limit_bytes: Option<u64>, active: &ActiveFolder, sink: &Sink) {
    let Some(limit) = limit_bytes else {
        return;
    };
    let cleanup = enforce(output_dir, limit, active);
    if cleanup.is_empty() {
        return;
    }
    tracing::info!(?cleanup, "storage limit cleanup");
    sink(UiMsg::Notice {
        level: NoticeLevel::Info,
        text: cleanup.notice(),
    });
    sink(UiMsg::Library(library::entries(output_dir)));
}

pub(crate) fn spawn(
    output_dir: PathBuf,
    limit_bytes: Option<u64>,
    active: ActiveFolder,
    sink: Sink,
) {
    if limit_bytes.is_none() {
        return;
    }
    let spawned = thread::Builder::new()
        .name("storage-limit".into())
        .spawn(move || run(&output_dir, limit_bytes, &active, &sink));
    if let Err(err) = spawned {
        tracing::error!(%err, "cannot start storage cleanup thread");
    }
}

#[cfg(test)]
mod tests;
