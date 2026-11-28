use std::fs;
use std::path::Path;

use greyscreen_core::disk::StorageUsage;
use greyscreen_core::library::META_FILE;

use crate::disk::{FreeSpace, SystemFreeSpace};
use crate::library::CLIPS_DIR;
use crate::storage::folder_bytes;
use crate::writer::VIDEO_FILE;

fn file_bytes(path: &Path) -> u64 {
    fs::symlink_metadata(path)
        .ok()
        .filter(|meta| meta.is_file())
        .map_or(0, |meta| meta.len())
}

fn clip_bytes(folder: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(folder.join(CLIPS_DIR)) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mp4"))
        .map(|path| file_bytes(&path))
        .sum()
}

pub(crate) fn scan(output_dir: &Path) -> StorageUsage {
    let mut usage = StorageUsage {
        free: SystemFreeSpace.free_bytes(output_dir),
        ..StorageUsage::default()
    };
    let Ok(entries) = fs::read_dir(output_dir) else {
        return usage;
    };
    for folder in entries.flatten().map(|entry| entry.path()) {
        if !folder.join(META_FILE).is_file() {
            continue;
        }
        let video = file_bytes(&folder.join(VIDEO_FILE));
        let clips = clip_bytes(&folder);
        usage.videos += video;
        usage.clips += clips;
        usage.previews += folder_bytes(&folder).saturating_sub(video + clips);
    }
    usage
}
