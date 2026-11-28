use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::library::thumb::{
    GAME_HERO_FILE, GAME_THUMB_FILE, clip_thumb_file, clip_thumb_secs, game_thumb_secs,
};
use greyscreen_core::library::{ClipEntry, RecordingMeta};

use crate::api::{self, RecordingEntry, UiMsg};
use crate::control::Sink;
use crate::storyboard::Queue;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Size {
    Thumb,
    Hero,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Job {
    pub(crate) size: Size,
    pub(crate) video: PathBuf,
    pub(crate) dest: PathBuf,
    pub(crate) at_secs: f64,
}

pub(crate) fn game_jobs(folder: &Path, meta: &RecordingMeta) -> Vec<Job> {
    if meta.video.is_empty() {
        return Vec::new();
    }
    [(Size::Thumb, GAME_THUMB_FILE), (Size::Hero, GAME_HERO_FILE)]
        .into_iter()
        .map(|(size, file)| Job {
            size,
            video: folder.join(&meta.video),
            dest: folder.join(file),
            at_secs: game_thumb_secs(meta),
        })
        .collect()
}

pub(crate) fn clip_job(folder: &Path, clip: &ClipEntry) -> Job {
    Job {
        size: Size::Thumb,
        video: folder.join(&clip.file),
        dest: folder.join(clip_thumb_file(clip)),
        at_secs: clip_thumb_secs(clip),
    }
}

pub(crate) fn jobs_for(entry: &RecordingEntry) -> Vec<Job> {
    game_jobs(&entry.folder, &entry.meta)
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

pub(crate) fn wrap(sink: Sink, storyboards: Queue) -> Sink {
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
            UiMsg::RecordingSaved(entry) => {
                storyboards.request(entry.folder.clone());
                jobs_for(entry)
            }
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
            let written = match job.size {
                Size::Thumb => {
                    greyscreen_thumbs::write_thumbnail(&job.video, job.at_secs, &job.dest)
                }
                Size::Hero => greyscreen_thumbs::write_hero(&job.video, job.at_secs, &job.dest),
            };
            match written {
                Ok(()) => sink(UiMsg::Thumbnail(job.dest)),
                Err(err) => {
                    tracing::warn!(%err, video = %job.video.display(), "no thumbnail");
                }
            }
        }
    }
}
