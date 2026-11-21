mod reencode;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use greyscreen_core::export::{
    DISCORD_LIMIT_BYTES, ExportDone, ExportEvent, ExportJob, ExportPreset, ExportRange,
    plan_discord, retry_plan,
};
use greyscreen_mp4::demux::Movie;
use greyscreen_mp4::trim::trim;

use crate::api::UiMsg;
use crate::control::Sink;
pub(crate) use reencode::{Reencode, ReencodeError, ReencodeRequest, default_reencoder};

pub const REENCODE_AVAILABLE: bool = reencode::AVAILABLE;

const PART_SUFFIX: &str = ".part";
const NO_SOURCE: &str =
    "The video for this part of the game was deleted to free up space, so it can't be exported.";

#[derive(Clone)]
pub(crate) struct ExportHandle {
    cancel: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    reencoder: Arc<dyn Reencode>,
}

impl std::fmt::Debug for ExportHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportHandle").finish_non_exhaustive()
    }
}

impl ExportHandle {
    pub(crate) fn new(reencoder: Arc<dyn Reencode>) -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
            reencoder,
        }
    }

    pub(crate) fn start(&self, job: ExportJob, sink: Sink) {
        if self.busy.swap(true, Ordering::SeqCst) {
            sink(UiMsg::Export(ExportEvent::Failed(
                "Another export is still running.".into(),
            )));
            return;
        }
        self.cancel.store(false, Ordering::SeqCst);
        let handle = self.clone();
        let worker_sink = sink.clone();
        let spawned = thread::Builder::new()
            .name("engine-export".into())
            .spawn(move || {
                let progress_sink = worker_sink.clone();
                let mut emit = move |event: ExportEvent| progress_sink(UiMsg::Export(event));
                let event = run(&job, handle.reencoder.as_ref(), &mut emit, &handle.cancel);
                handle.busy.store(false, Ordering::SeqCst);
                worker_sink(UiMsg::Export(event));
            });
        if let Err(err) = spawned {
            tracing::warn!(%err, "cannot start the export thread");
            self.busy.store(false, Ordering::SeqCst);
            sink(UiMsg::Export(ExportEvent::Failed(
                "Couldn't start the export. Try again.".into(),
            )));
        }
    }

    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

pub(crate) fn run(
    job: &ExportJob,
    reencoder: &dyn Reencode,
    emit: &mut dyn FnMut(ExportEvent),
    cancel: &AtomicBool,
) -> ExportEvent {
    if !job.source.is_file() {
        return ExportEvent::Failed(NO_SOURCE.into());
    }
    if let Err(err) = fs::create_dir_all(&job.dest_dir) {
        tracing::warn!(%err, dir = %job.dest_dir.display(), "cannot create the exports folder");
        return ExportEvent::Failed(format!(
            "Couldn't use the folder {}.",
            job.dest_dir.display()
        ));
    }
    let part = job.dest_dir.join(format!("{}.mp4{PART_SUFFIX}", job.stem));
    emit(ExportEvent::Progress(0.0));
    let outcome = match job.preset {
        ExportPreset::Original => cut_original(job, &part),
        ExportPreset::Discord => encode_discord(job, &part, reencoder, emit, cancel),
    };
    let finished = outcome.and_then(|(range, shrunk)| {
        if cancel.load(Ordering::SeqCst) {
            return Err(Stop::Cancelled);
        }
        let path = unique_path(&job.dest_dir, &job.stem);
        fs::rename(&part, &path).map_err(|err| Stop::Failed(file_error(&err)))?;
        let bytes = fs::metadata(&path).map_or(0, |meta| meta.len());
        Ok(ExportDone {
            path,
            bytes,
            range,
            preset: job.preset,
            shrunk,
        })
    });
    match finished {
        Ok(done) => ExportEvent::Done(done),
        Err(stop) => {
            fs::remove_file(&part).ok();
            match stop {
                Stop::Cancelled => ExportEvent::Cancelled,
                Stop::Failed(text) => ExportEvent::Failed(text),
            }
        }
    }
}

enum Stop {
    Cancelled,
    Failed(String),
}

type Cut = Result<(ExportRange, bool), Stop>;

fn source_range(job: &ExportJob) -> (f64, f64) {
    (
        (job.range.start - job.source_offset_secs).max(0.0),
        (job.range.end - job.source_offset_secs).max(0.0),
    )
}

fn cut_original(job: &ExportJob, part: &Path) -> Cut {
    let (start, end) = source_range(job);
    let trimmed = trim(&job.source, part, start, end).map_err(|err| {
        tracing::warn!(%err, "cannot cut the video");
        Stop::Failed("Couldn't cut that part of the video.".into())
    })?;
    let range = ExportRange::new(
        trimmed.start_secs + job.source_offset_secs,
        trimmed.end_secs + job.source_offset_secs,
    );
    Ok((range, false))
}

fn encode_discord(
    job: &ExportJob,
    part: &Path,
    reencoder: &dyn Reencode,
    emit: &mut dyn FnMut(ExportEvent),
    cancel: &AtomicBool,
) -> Cut {
    let (start, wanted_end) = source_range(job);
    let movie = Movie::open(&job.source).map_err(|err| {
        tracing::warn!(%err, "cannot read the video");
        Stop::Failed("Couldn't read the video.".into())
    })?;
    let end = wanted_end.min(movie.video_secs());
    if end - start < greyscreen_core::export::MIN_SECS / 2.0 {
        return Err(Stop::Failed("That part of the video is empty.".into()));
    }
    let secs = end - start;
    let mut plan = plan_discord(secs, job.video).map_err(|err| Stop::Failed(err.to_string()))?;
    let mut shrunk = false;
    loop {
        let request = ReencodeRequest {
            src: &job.source,
            dst: part,
            start_secs: start,
            end_secs: end,
            plan,
        };
        let mut progress = |fraction: f32| emit(ExportEvent::Progress(fraction));
        reencoder
            .run(&request, &mut progress, cancel)
            .map_err(|err| match err {
                ReencodeError::Cancelled => Stop::Cancelled,
                ReencodeError::Unsupported => Stop::Failed(
                    "Only Original quality works on this computer. The Discord preset needs Windows."
                        .into(),
                ),
                ReencodeError::Failed(why) => {
                    tracing::warn!(%why, "re-encode failed");
                    Stop::Failed("Couldn't make the smaller file. Try Original quality.".into())
                }
            })?;
        let bytes = fs::metadata(part).map_or(0, |meta| meta.len());
        if bytes <= DISCORD_LIMIT_BYTES {
            let range =
                ExportRange::new(start + job.source_offset_secs, end + job.source_offset_secs);
            return Ok((range, shrunk));
        }
        if shrunk {
            return Err(Stop::Failed(
                "Couldn't get this under 10 MB. Try a shorter clip.".into(),
            ));
        }
        tracing::info!(bytes, "export over the limit, trying a smaller size");
        plan = retry_plan(plan, bytes, secs, job.video)
            .map_err(|err| Stop::Failed(err.to_string()))?;
        shrunk = true;
        fs::remove_file(part).ok();
        if cancel.load(Ordering::SeqCst) {
            return Err(Stop::Cancelled);
        }
        emit(ExportEvent::Shrinking);
    }
}

fn unique_path(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.mp4"));
    if !first.exists() {
        return first;
    }
    (2u32..)
        .map(|n| dir.join(format!("{stem} ({n}).mp4")))
        .find(|path| !path.exists())
        .unwrap_or(first)
}

fn file_error(err: &std::io::Error) -> String {
    tracing::warn!(%err, "cannot save the export");
    "Couldn't save the file. Check that the folder is writable and the drive has space.".into()
}
