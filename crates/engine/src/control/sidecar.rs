use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::library::{self, LibraryError, RecordingMeta};

struct Job {
    epoch: u64,
    folder: PathBuf,
    meta: RecordingMeta,
}

type WriteFn = Arc<dyn Fn(&Path, &RecordingMeta) + Send + Sync>;

struct Shared {
    epoch: AtomicU64,
    gate: Mutex<()>,
    write: WriteFn,
}

pub(super) struct SidecarWriter {
    tx: Option<Sender<Job>>,
    thread: Option<JoinHandle<()>>,
    shared: Arc<Shared>,
}

impl SidecarWriter {
    pub(super) fn spawn() -> Self {
        Self::spawn_with(Arc::new(write_now))
    }

    fn spawn_with(write: WriteFn) -> Self {
        let shared = Arc::new(Shared {
            epoch: AtomicU64::new(0),
            gate: Mutex::new(()),
            write,
        });
        let (tx, rx) = unbounded::<Job>();
        let seen = Arc::clone(&shared);
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
                shared,
            },
            Err(err) => {
                tracing::error!(%err, "cannot start sidecar write thread");
                Self {
                    tx: None,
                    thread: None,
                    shared,
                }
            }
        }
    }

    pub(super) fn queue(&self, folder: PathBuf, meta: RecordingMeta) {
        let epoch = self.shared.epoch.load(Ordering::SeqCst);
        let Some(tx) = &self.tx else {
            (self.shared.write)(&folder, &meta);
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
        folder: &Path,
        meta: &RecordingMeta,
    ) -> Result<(), LibraryError> {
        let _gate = self.shared.gate.lock();
        self.shared.epoch.fetch_add(1, Ordering::SeqCst);
        library::write_meta(folder, meta)
    }
}

fn write_if_current(shared: &Shared, job: &Job) {
    let Ok(_gate) = shared.gate.lock() else {
        return;
    };
    if job.epoch < shared.epoch.load(Ordering::SeqCst) {
        return;
    }
    (shared.write)(&job.folder, &job.meta);
}

fn write_now(folder: &Path, meta: &RecordingMeta) {
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
