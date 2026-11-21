#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::library::thumb::{
    GAME_THUMB_FILE, clip_thumb_file, clip_thumb_secs, game_thumb_secs,
};
use greyscreen_core::library::{ClipEntry, RecordingMeta};

use crate::api::{self, RecordingEntry, UiMsg};
use crate::control::Sink;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Job {
    pub(crate) video: PathBuf,
    pub(crate) dest: PathBuf,
    pub(crate) at_secs: f64,
}

pub(crate) fn game_job(folder: &Path, meta: &RecordingMeta) -> Option<Job> {
    (!meta.video.is_empty()).then(|| Job {
        video: folder.join(&meta.video),
        dest: folder.join(GAME_THUMB_FILE),
        at_secs: game_thumb_secs(meta),
    })
}

pub(crate) fn clip_job(folder: &Path, clip: &ClipEntry) -> Job {
    Job {
        video: folder.join(&clip.file),
        dest: folder.join(clip_thumb_file(clip)),
        at_secs: clip_thumb_secs(clip),
    }
}

pub(crate) fn jobs_for(entry: &RecordingEntry) -> Vec<Job> {
    game_job(&entry.folder, &entry.meta)
        .into_iter()
        .chain(
            entry
                .meta
                .clips
                .iter()
                .map(|clip| clip_job(&entry.folder, clip)),
        )
        .collect()
}

pub(crate) fn wrap(sink: Sink) -> Sink {
    let (jobs, queue) = unbounded();
    let notify = sink.clone();
    let spawned = thread::Builder::new()
        .name("thumbnails".into())
        .spawn(move || run(&queue, &notify));
    if let Err(err) = spawned {
        tracing::error!(%err, "cannot start thumbnail thread");
        return sink;
    }
    Arc::new(move |msg| {
        let wanted = match &msg {
            UiMsg::Library(entries) => entries.iter().flat_map(jobs_for).collect(),
            UiMsg::RecordingSaved(entry) => jobs_for(entry),
            UiMsg::ClipSaved(api::ClipEntry { clip, folder }) => vec![clip_job(folder, clip)],
            _ => Vec::new(),
        };
        sink(msg);
        if !wanted.is_empty() && jobs.send(wanted).is_err() {
            tracing::debug!("thumbnail thread stopped");
        }
    })
}

fn run(queue: &Receiver<Vec<Job>>, sink: &Sink) {
    let mut seen = HashSet::new();
    for batch in queue.iter() {
        for job in batch {
            if !seen.insert(job.dest.clone()) || job.dest.is_file() {
                continue;
            }
            match greyscreen_thumbs::write_thumbnail(&job.video, job.at_secs, &job.dest) {
                Ok(()) => sink(UiMsg::Thumbnail(job.dest)),
                Err(err) => {
                    tracing::warn!(%err, video = %job.video.display(), "no thumbnail");
                }
            }
        }
    }
}
