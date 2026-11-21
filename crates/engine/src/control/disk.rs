use std::time::Instant;

use greyscreen_core::disk::{self, START_FLOOR_BYTES};
use greyscreen_core::media::CaptureConfig;
use greyscreen_core::session::{Input, SessionState};

use super::effects::send_internal;
use super::{Control, Internal};
use crate::api::{NoticeLevel, UiMsg};
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
        if self.disk_checked.is_some_and(|at| at.elapsed() < every) {
            return;
        }
        self.disk_checked = Some(Instant::now());
        let Some(free) = self.free_bytes().filter(|free| disk::must_stop(*free)) else {
            return;
        };
        let free = self.make_room(free, START_FLOOR_BYTES);
        if !disk::must_stop(free) {
            return;
        }
        tracing::error!(free, "the disk is almost full, stopping the recording");
        self.send(messages::stopped(free));
        self.feed(Input::UserStop);
    }

    fn free_bytes(&self) -> Option<u64> {
        self.free_space.free_bytes(&self.settings.output_dir)
    }

    fn make_room(&mut self, free: u64, wanted: u64) -> u64 {
        if self.settings.storage_limit_gb.is_none() {
            return free;
        }
        let output_dir = self.settings.output_dir.clone();
        let cleanup = storage::free_up(
            &output_dir,
            wanted.saturating_sub(free),
            &self.active_folder,
        );
        if cleanup.videos == 0 {
            return free;
        }
        tracing::info!(?cleanup, "deleted old game videos to make room");
        self.notice(NoticeLevel::Info, cleanup.low_disk_notice());
        self.send(UiMsg::Library(library::entries(&output_dir)));
        self.free_bytes().unwrap_or(free)
    }
}
