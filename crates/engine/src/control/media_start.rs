use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;
use std::time::Instant;

use crossbeam_channel::{Receiver, SendError, bounded};
use greyscreen_core::media::{CaptureTarget, MediaEvent, MediaSession};
use greyscreen_core::problem::Problem;
use greyscreen_core::session::Input;
use greyscreen_core::settings::NATIVE_HEIGHT_CEILING;

use super::effects::send_internal;
use super::{Control, Internal, folder};
use crate::api::{NoticeLevel, UiMsg};

const MEDIA_QUEUE: usize = 1024;
const START_TIMED_OUT: &str = "capture did not start";
const START_CRASHED: &str = "The capture session crashed while starting.";

pub(super) struct PendingStart {
    id: u64,
    deadline: Instant,
    abandoned: bool,
}

impl Control {
    pub(super) fn start_media(&mut self, target: CaptureTarget) {
        if self.start.is_some() {
            self.queued_start = Some(target);
            return;
        }
        self.begin_start(target, None);
    }

    fn begin_start(&mut self, target: CaptureTarget, leftover: Option<Box<dyn MediaSession>>) {
        let cfg = self.settings.capture_config(NATIVE_HEIGHT_CEILING);
        if !self.disk_allows_start(&cfg) {
            if let Some(session) = leftover {
                self.stop_unwanted(session);
            }
            return;
        }
        self.start_ids += 1;
        let id = self.start_ids;
        let (tx, rx) = bounded(MEDIA_QUEUE);
        let media = self.media.clone();
        let dir = self.settings.output_dir.clone();
        let internal = self.internal_tx.clone();
        let spawned = thread::Builder::new()
            .name("media-start".into())
            .spawn(move || {
                if let Some(session) = leftover {
                    session.stop();
                }
                let started = catch_unwind(AssertUnwindSafe(|| {
                    folder::check_writable(&dir).and_then(|()| media.start(target, cfg, tx))
                }))
                .unwrap_or_else(|_| Err(START_CRASHED.to_owned()));
                let msg = Internal::MediaStart {
                    id,
                    result: started,
                    rx,
                };
                if let Err(SendError(Internal::MediaStart {
                    result: Ok(session),
                    ..
                })) = internal.send(msg)
                {
                    session.stop();
                }
            });
        match spawned {
            Ok(_) => {
                self.start = Some(PendingStart {
                    id,
                    deadline: Instant::now() + self.start_deadline,
                    abandoned: false,
                });
            }
            Err(err) => {
                tracing::error!(%err, "cannot start media start thread");
                self.report_start_failure(&err.to_string());
            }
        }
    }

    pub(super) fn on_media_start(
        &mut self,
        id: u64,
        result: Result<Box<dyn MediaSession>, String>,
        rx: Receiver<MediaEvent>,
    ) {
        let Some(pending) = self.start.take_if(|pending| pending.id == id) else {
            return;
        };
        let stale = pending.abandoned || self.queued_start.is_some();
        let mut leftover = None;
        match result {
            Ok(session) if stale => {
                tracing::warn!("capture started too late, stopping it");
                leftover = Some(session);
            }
            Ok(session) => {
                self.media_gen += 1;
                self.media_session = Some(session);
                self.media_rx = rx;
                self.stopping = false;
            }
            Err(err) if stale => tracing::warn!(%err, "late media start failed"),
            Err(err) => self.report_start_failure(&err),
        }
        match self.queued_start.take() {
            Some(target) => self.begin_start(target, leftover),
            None => {
                if let Some(session) = leftover {
                    self.stop_unwanted(session);
                }
            }
        }
    }

    pub(super) fn on_unwanted_stopped(&mut self) {
        self.unwanted_stops = self.unwanted_stops.saturating_sub(1);
    }

    fn stop_unwanted(&mut self, session: Box<dyn MediaSession>) {
        let sink = self.sink.clone();
        let done = self.internal_tx.clone();
        let spawned = thread::Builder::new()
            .name("media-stop".into())
            .spawn(move || {
                if catch_unwind(AssertUnwindSafe(|| session.stop())).is_err() {
                    tracing::error!("media session panicked while stopping");
                    sink(UiMsg::Notice {
                        level: NoticeLevel::Error,
                        text: "The capture session crashed while stopping.".to_owned(),
                    });
                }
                send_internal(&done, Internal::UnwantedStopped);
            });
        match spawned {
            Ok(_) => self.unwanted_stops += 1,
            Err(err) => tracing::error!(%err, "cannot start media stop thread"),
        }
    }

    pub(super) fn abandon_start(&mut self) {
        self.queued_start = None;
        if let Some(pending) = self.start.as_mut() {
            pending.abandoned = true;
        }
    }

    pub(super) fn check_start_deadline(&mut self) {
        let Some(pending) = self.start.as_mut() else {
            return;
        };
        if pending.abandoned || Instant::now() < pending.deadline {
            return;
        }
        pending.abandoned = true;
        if self.queued_start.is_none() {
            self.report_start_failure(START_TIMED_OUT);
        }
    }

    fn report_start_failure(&mut self, err: &str) {
        tracing::error!(%err, "media start failed");
        self.notice(
            NoticeLevel::Error,
            Problem::notice("Capture failed to start", err),
        );
        self.feed(Input::MediaFailed(err.to_owned()));
    }
}
