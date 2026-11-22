use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::library::{self, LibraryError, RecordingMeta};

struct Job {
    epoch: u64,
    folder: PathBuf,
    meta: RecordingMeta,
}

#[derive(Debug)]
pub(super) struct SidecarWriter {
    tx: Option<Sender<Job>>,
    thread: Option<JoinHandle<()>>,
    epoch: Arc<Mutex<u64>>,
}

impl SidecarWriter {
    pub(super) fn spawn() -> Self {
        let epoch = Arc::new(Mutex::new(0));
        let (tx, rx) = unbounded::<Job>();
        let seen = Arc::clone(&epoch);
        let spawned = thread::Builder::new()
            .name("sidecar-write".into())
            .spawn(move || {
                while let Ok(mut job) = rx.recv() {
                    while let Ok(newer) = rx.try_recv() {
                        if newer.folder == job.folder {
                            job = newer;
                        } else {
                            write_if_current(&seen, &job);
                            job = newer;
                        }
                    }
                    write_if_current(&seen, &job);
                }
            });
        match spawned {
            Ok(thread) => Self {
                tx: Some(tx),
                thread: Some(thread),
                epoch,
            },
            Err(err) => {
                tracing::error!(%err, "cannot start sidecar write thread");
                Self {
                    tx: None,
                    thread: None,
                    epoch,
                }
            }
        }
    }

    pub(super) fn queue(&self, folder: PathBuf, meta: RecordingMeta) {
        let epoch = self.epoch.lock().map_or(0, |epoch| *epoch);
        let Some(tx) = &self.tx else {
            write_now(&folder, &meta);
            return;
        };
        if tx
            .send(Job {
                epoch,
                folder,
                meta,
            })
            .is_err()
        {
            tracing::warn!("sidecar writer is gone");
        }
    }

    pub(super) fn write_final(
        &self,
        folder: &std::path::Path,
        meta: &RecordingMeta,
    ) -> Result<(), LibraryError> {
        let Ok(mut epoch) = self.epoch.lock() else {
            return library::write_meta(folder, meta);
        };
        *epoch += 1;
        library::write_meta(folder, meta)
    }
}

fn write_if_current(epoch: &Mutex<u64>, job: &Job) {
    let Ok(epoch) = epoch.lock() else {
        return;
    };
    if job.epoch < *epoch {
        return;
    }
    write_now(&job.folder, &job.meta);
}

fn write_now(folder: &std::path::Path, meta: &RecordingMeta) {
    if let Err(err) = library::write_meta(folder, meta) {
        tracing::warn!(%err, "cannot write recording sidecar");
    }
}

impl Drop for SidecarWriter {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

#[cfg(test)]
mod tests;
