use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use greyscreen_core::library::ClipEntry;
use greyscreen_core::media::{AudioFormat, Packet, VideoFormat};
use greyscreen_mp4::clip::write_clip;

pub(crate) type ClipResult = Result<ClipEntry, String>;

pub(crate) struct ClipJob {
    pub(crate) path: PathBuf,
    pub(crate) entry: ClipEntry,
    pub(crate) video: VideoFormat,
    pub(crate) audio: Option<AudioFormat>,
    pub(crate) packets: Vec<Arc<Packet>>,
}

pub(crate) fn spawn(job: ClipJob, on_done: impl FnOnce(ClipResult) + Send + 'static) -> bool {
    spawn_with(job, write, on_done)
}

fn spawn_with(
    job: ClipJob,
    write: impl FnOnce(&ClipJob) -> ClipResult + Send + 'static,
    on_done: impl FnOnce(ClipResult) + Send + 'static,
) -> bool {
    let spawned = thread::Builder::new()
        .name("clip-writer".into())
        .spawn(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| write(&job)));
            on_done(outcome.unwrap_or_else(|_| Err("clip writer panicked".to_owned())));
        });
    match spawned {
        Ok(_) => true,
        Err(err) => {
            tracing::error!(%err, "cannot start clip writer thread");
            false
        }
    }
}

fn write(job: &ClipJob) -> ClipResult {
    if let Some(dir) = job.path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    write_clip(&job.path, &job.video, job.audio.as_ref(), &job.packets)
        .map_err(|e| format!("{}: {e}", job.entry.file))?;
    Ok(job.entry.clone())
}

#[cfg(test)]
mod tests;
