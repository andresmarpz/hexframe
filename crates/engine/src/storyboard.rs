#[cfg(test)]
mod tests;

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError, unbounded};
use greyscreen_core::library::storyboard::{self, Storyboard};
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_thumbs::{Pass, SheetLayout};

use crate::api::UiMsg;
use crate::control::Sink;
use crate::storage::ActiveFolder;

#[cfg(not(test))]
const RETRY: Duration = Duration::from_secs(2);
#[cfg(test)]
const RETRY: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub(crate) struct Queue(Sender<PathBuf>);

impl Queue {
    pub(crate) fn request(&self, folder: PathBuf) {
        if self.0.send(folder).is_err() {
            tracing::debug!("storyboard thread stopped");
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Built,
    Stopped,
    Skipped,
    Failed(String),
}

pub(crate) fn spawn(sink: Sink, active: ActiveFolder) -> Queue {
    let (requests, queue) = unbounded();
    let spawned = thread::Builder::new()
        .name("storyboards".into())
        .spawn(move || run(&queue, &sink, &active));
    if let Err(err) = spawned {
        tracing::error!(%err, "cannot start storyboard thread");
    }
    Queue(requests)
}

fn recording(active: &ActiveFolder) -> bool {
    active.lock().is_ok_and(|id| id.is_some())
}

fn run(queue: &Receiver<PathBuf>, sink: &Sink, active: &ActiveFolder) {
    greyscreen_thumbs::lower_current_thread_priority();
    let mut pending: VecDeque<PathBuf> = VecDeque::new();
    let mut failed: HashSet<PathBuf> = HashSet::new();
    loop {
        let ready = !pending.is_empty() && !recording(active);
        let next = if ready {
            queue.try_recv().map_err(|err| match err {
                TryRecvError::Empty => RecvTimeoutError::Timeout,
                TryRecvError::Disconnected => RecvTimeoutError::Disconnected,
            })
        } else if pending.is_empty() {
            queue.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            queue.recv_timeout(RETRY)
        };
        match next {
            Ok(folder) => {
                if !failed.contains(&folder) && !pending.contains(&folder) {
                    pending.push_back(folder);
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {}
        }
        if !ready {
            continue;
        }
        let Some(folder) = pending.pop_front() else {
            continue;
        };
        let stop = || recording(active) || !folder.is_dir();
        match build(&folder, &stop) {
            Outcome::Built => sink(UiMsg::Storyboard(folder)),
            Outcome::Stopped if folder.is_dir() => pending.push_front(folder),
            Outcome::Stopped | Outcome::Skipped => {}
            Outcome::Failed(reason) => {
                tracing::warn!(%reason, folder = %folder.display(), "no storyboard");
                failed.insert(folder);
            }
        }
    }
}

pub fn build_storyboard(folder: &Path) -> Result<u32, String> {
    match build(folder, &|| false) {
        Outcome::Built => storyboard::read(folder)
            .map(|board| board.frames)
            .ok_or_else(|| "the index cannot be read back".to_owned()),
        Outcome::Skipped => Err("already built, or no video, or too short".to_owned()),
        Outcome::Stopped => Err("stopped".to_owned()),
        Outcome::Failed(reason) => Err(reason),
    }
}

fn source(folder: &Path) -> Option<(PathBuf, RecordingMeta)> {
    let meta = library::read_meta(folder).ok()?;
    let video = folder.join(&meta.video);
    (!meta.video.is_empty() && video.is_file()).then_some((video, meta))
}

pub(crate) fn build(folder: &Path, stop: &dyn Fn() -> bool) -> Outcome {
    if storyboard::read(folder).is_some() {
        return Outcome::Skipped;
    }
    let Some((video, meta)) = source(folder) else {
        return Outcome::Skipped;
    };
    let Some(plan) = Storyboard::plan(meta.duration_secs) else {
        return Outcome::Skipped;
    };
    let times: Vec<f64> = plan.times().collect();
    let layout = SheetLayout {
        tile_width: plan.tile_width,
        tile_height: plan.tile_height,
        columns: plan.columns,
        rows: plan.rows,
    };
    let dest = |index: usize| folder.join(storyboard::sheet_file(index));
    let pass = greyscreen_thumbs::write_sheets(&video, &times, layout, &dest, stop);
    let outcome = match pass {
        Ok(Pass::Finished { sheets }) if sheets == plan.sheets.len() => {
            match storyboard::write(folder, &plan) {
                Ok(()) => Outcome::Built,
                Err(err) => Outcome::Failed(err.to_string()),
            }
        }
        Ok(Pass::Finished { sheets }) => Outcome::Failed(format!("wrote {sheets} sheets")),
        Ok(Pass::Stopped) => Outcome::Stopped,
        Err(err) => Outcome::Failed(err.to_string()),
    };
    if outcome != Outcome::Built && folder.is_dir() {
        storyboard::remove(folder);
    }
    outcome
}
