use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use greyscreen_core::library;

use crate::api::{RecordingEntry, UiMsg};
use crate::control::Sink;
use crate::recovery;
use crate::writer::VIDEO_FILE;

pub(crate) const CLIPS_DIR: &str = "clips";

pub(crate) fn size_bytes(folder: &Path) -> u64 {
    let file = fs::metadata(folder.join(VIDEO_FILE)).map_or(0, |m| m.len());
    let clips = fs::read_dir(folder.join(CLIPS_DIR)).map_or(0, |entries| {
        entries
            .flatten()
            .filter_map(|entry| entry.metadata().ok())
            .filter(|meta| meta.is_file())
            .map(|meta| meta.len())
            .sum()
    });
    file + clips
}

pub(crate) fn entries(output_dir: &Path) -> Vec<RecordingEntry> {
    library::scan(output_dir)
        .into_iter()
        .filter_map(|meta| {
            let folder = output_dir.join(&meta.id);
            folder.is_dir().then(|| RecordingEntry {
                size_bytes: size_bytes(&folder),
                meta,
                folder,
            })
        })
        .collect()
}

pub(crate) fn spawn_scan(output_dir: PathBuf, sink: Sink) {
    spawn_worker("library-scan", move || {
        sink(UiMsg::Library(entries(&output_dir)));
    });
}

pub(crate) fn spawn_startup(output_dir: PathBuf, sink: Sink) {
    let parts = recovery::find_parts(&output_dir);
    spawn_worker("library-startup", move || {
        for part in &parts {
            recovery::recover_part(part);
        }
        sink(UiMsg::Library(entries(&output_dir)));
    });
}

fn spawn_worker(name: &str, work: impl FnOnce() + Send + 'static) {
    if let Err(err) = thread::Builder::new().name(name.into()).spawn(work) {
        tracing::error!(%err, name, "cannot start worker thread");
    }
}
