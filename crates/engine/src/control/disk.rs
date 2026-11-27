use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossbeam_channel::Sender;
use greyscreen_core::disk::{self, START_FLOOR_BYTES};
use greyscreen_core::media::CaptureConfig;
use greyscreen_core::session::{Input, SessionState};

use super::effects::send_internal;
use super::{Control, Internal};
use crate::api::{NoticeLevel, RecordingEntry, UiMsg};
use crate::disk::FreeSpace;
use crate::storage::ActiveFolder;
use crate::{disk as messages, library, storage};

impl Control {
    pub(super) fn disk_allows_start(&mut self, config: &CaptureConfig) -> bool {
        let Some(free) = self.free_bytes() else {
            return true;
        };
        let rate = disk::bytes_per_sec(config);
        let free = if disk::can_start(free, rate) {
            free
        } else {
            self.make_room(free, disk::start_threshold(rate))
        };
        if disk::can_start(free, rate) {
            self.disk_refused = false;
            return true;
        }
        tracing::warn!(free, "not enough disk space to start recording");
        if !self.disk_refused {
            self.disk_refused = true;
            self.send(messages::refused(free));
        }
        let reason = messages::REFUSED_REASON.to_owned();
        send_internal(&self.internal_tx, Internal::StartFailed(reason));
        false
    }

    pub(super) fn check_disk_while_recording(&mut self) {
        let recording = self.recording.is_some()
            && matches!(self.session.state(), SessionState::Recording { .. });
        if !recording {
            self.disk_checked = None;
            return;
        }
        let every = self.free_space.check_every();
        if self.disk_check_running || self.disk_checked.is_some_and(|at| at.elapsed() < every) {
            return;
        }
        self.disk_checked = Some(Instant::now());
        self.disk_check_running = true;
        let space = Arc::clone(&self.free_space);
        let dir = self.settings.output_dir.clone();
        let can_clean = self.settings.storage_limit_gb.is_some();
        let active = Arc::clone(&self.active_folder);
        let reply = ReplyOnDrop(Some(self.internal_tx.clone()));
        let spawned = thread::Builder::new()
            .name("disk-check".to_owned())
            .spawn(move || reply.send(run_check(space.as_ref(), &dir, can_clean, &active)));
        if spawned.is_err() {
            self.disk_check_running = false;
        }
    }

    pub(super) fn on_disk_free(&mut self, check: DiskCheck) {
        self.disk_check_running = false;
        let recording = self.recording.is_some()
            && matches!(self.session.state(), SessionState::Recording { .. });
        if let Some(cleaned) = check.cleaned {
            self.announce_cleanup(cleaned);
        }
        let Some(free) = check
            .free
            .filter(|free| recording && disk::must_stop(*free))
        else {
            return;
        };
        tracing::error!(free, "the disk is almost full, stopping the recording");
        self.send(messages::stopped(free));
        self.feed(Input::UserStop);
    }

    fn announce_cleanup(&mut self, cleaned: Cleaned) {
        tracing::info!(cleanup = ?cleaned.cleanup, "deleted old game videos to make room");
        self.notice(NoticeLevel::Info, cleaned.cleanup.low_disk_notice());
        self.send(UiMsg::Library(cleaned.entries));
    }

    fn free_bytes(&self) -> Option<u64> {
        self.free_space.free_bytes(&self.settings.output_dir)
    }

    fn make_room(&mut self, free: u64, wanted: u64) -> u64 {
        let reclaimed = reclaim(
            self.free_space.as_ref(),
            &self.settings.output_dir,
            self.settings.storage_limit_gb.is_some(),
            &self.active_folder,
            free,
            wanted,
        );
        if let Some(cleaned) = reclaimed.cleaned {
            self.announce_cleanup(cleaned);
        }
        reclaimed.free
    }
}

pub(super) struct Cleaned {
    cleanup: storage::Cleanup,
    entries: Vec<RecordingEntry>,
}

pub(crate) struct DiskCheck {
    free: Option<u64>,
    cleaned: Option<Cleaned>,
}

struct Reclaimed {
    free: u64,
    cleaned: Option<Cleaned>,
}

fn reclaim(
    space: &dyn FreeSpace,
    dir: &Path,
    can_clean: bool,
    active: &ActiveFolder,
    free: u64,
    wanted: u64,
) -> Reclaimed {
    let unchanged = Reclaimed {
        free,
        cleaned: None,
    };
    if !can_clean {
        return unchanged;
    }
    let cleanup = storage::free_up(dir, wanted.saturating_sub(free), active);
    if cleanup.videos == 0 {
        return unchanged;
    }
    let entries = library::entries(dir);
    Reclaimed {
        free: space.free_bytes(dir).unwrap_or(free),
        cleaned: Some(Cleaned { cleanup, entries }),
    }
}

fn run_check(
    space: &dyn FreeSpace,
    dir: &Path,
    can_clean: bool,
    active: &ActiveFolder,
) -> DiskCheck {
    let Some(free) = space.free_bytes(dir) else {
        return DiskCheck {
            free: None,
            cleaned: None,
        };
    };
    if !disk::must_stop(free) {
        return DiskCheck {
            free: Some(free),
            cleaned: None,
        };
    }
    let reclaimed = reclaim(space, dir, can_clean, active, free, START_FLOOR_BYTES);
    DiskCheck {
        free: Some(reclaimed.free),
        cleaned: reclaimed.cleaned,
    }
}

struct ReplyOnDrop(Option<Sender<Internal>>);

impl ReplyOnDrop {
    fn send(mut self, check: DiskCheck) {
        if let Some(tx) = self.0.take() {
            send_internal(&tx, Internal::DiskFree(check));
        }
    }
}

impl Drop for ReplyOnDrop {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            send_internal(
                &tx,
                Internal::DiskFree(DiskCheck {
                    free: None,
                    cleaned: None,
                }),
            );
        }
    }
}
