use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;

use greyscreen_core::clips::{DUE_DELAY, MAX_PLAN};
use greyscreen_core::encoder;
use greyscreen_core::game::{GameInfo, GameResult, TimedEvent};
use greyscreen_core::gamedata;
use greyscreen_core::highlight::classify;
use greyscreen_core::library::EventEntry;
use greyscreen_core::media::MediaEvent;
use greyscreen_core::problem::Problem;
use greyscreen_core::session::{Effect, Input, SessionState, Status};
use greyscreen_core::time::{Hns, hns_to_secs, mono_now, secs_to_hns};

use super::meta;
use super::{Control, Internal, StartedMedia};
use crate::api::{FeedItem, NoticeLevel, RecordingEntry, UiMsg};
use crate::diagnostics::DiagWriter;
use crate::library;
use crate::storage;
use crate::writer::{GameWriter, VIDEO_FILE};

const RING_MARGIN_SECS: u32 = 5;
pub(super) const DISK_TOO_SLOW: &str = "Disk too slow: the full game recording stopped";
const ENCODER_STOPPED: &str = "Recording stopped: the video encoder or graphics driver failed. The game is saved up to that point.";

pub(super) fn ring_keep() -> Hns {
    let plan = MAX_PLAN + DUE_DELAY;
    let manual = secs_to_hns(f64::from(super::clips::MANUAL_CLIP_SECS));
    plan.max(manual) + secs_to_hns(f64::from(RING_MARGIN_SECS))
}

impl Control {
    pub(super) fn run_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::StartMedia(target) => self.start_media(target),
                Effect::StopMedia => self.stop_media(),
                Effect::Finalize { result } => self.finalize(result),
                Effect::Highlight(event) => self.on_highlight(event),
                Effect::Status(Status::Recording { since, game }) => {
                    self.on_recording_status(since, game);
                }
                Effect::Status(status) => self.send(UiMsg::Status(status)),
            }
        }
    }

    fn on_recording_status(&mut self, since: Hns, game: Option<GameInfo>) {
        let status = Status::Recording {
            since,
            game: game.clone(),
        };
        if let Err(err) = self.begin_recording(since, game, status) {
            self.fail_recording_start(&err);
        }
    }

    pub(super) fn fail_recording_start(&mut self, err: &str) {
        tracing::error!(%err, "cannot begin recording");
        self.notice(
            NoticeLevel::Error,
            Problem::notice("Cannot start recording", err),
        );
        self.feed(Input::MediaFailed(err.to_owned()));
    }

    fn stop_media(&mut self) {
        self.abandon_start();
        self.stopping = true;
        let generation = self.media_gen;
        let tx = self.internal_tx.clone();
        let Some(session) = self.media_session.take() else {
            send_internal(&tx, Internal::MediaStopped(generation));
            return;
        };
        let sink = self.sink.clone();
        let done = tx.clone();
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
                send_internal(&done, Internal::MediaStopped(generation));
            });
        if let Err(err) = spawned {
            tracing::error!(%err, "cannot start media stop thread");
            send_internal(&tx, Internal::MediaStopped(generation));
        }
    }

    fn finalize(&mut self, result: GameResult) {
        match self.recording.as_mut() {
            Some(recording) => recording.result = Some(result),
            None => match self.folder_start.as_mut() {
                Some(pending) => pending.result = Some(result),
                None => self.orphan_finalize = true,
            },
        }
    }

    pub(super) fn on_highlight(&mut self, event: TimedEvent) {
        if let Some(pending) = self.folder_start.as_mut() {
            pending.highlights.push(event);
            return;
        }
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let rec_secs = recording.file_secs(event.at);
        let highlights = classify(&event.kind, &recording.meta.game.info);
        gamedata::report_unmapped_event(&event.kind, &recording.meta.game.info);
        if rec_secs >= 0.0 {
            recording.meta.events.push(EventEntry {
                id: event.id,
                game_secs: event.game_secs,
                rec_secs,
                kind: event.kind.clone(),
                highlights: highlights.iter().map(|h| h.label().to_owned()).collect(),
            });
            recording.dirty = true;
        }
        self.plan_clip(event.at, event.id, &highlights);
        let item: FeedItem = meta::feed_item(&event, rec_secs, highlights);
        self.send(UiMsg::Feed(item));
    }

    pub(super) fn on_media(&mut self, event: MediaEvent) {
        match event {
            MediaEvent::Started {
                t0,
                video,
                audio,
                encoder,
            } => {
                self.started = Some(StartedMedia {
                    video,
                    audio,
                    encoder,
                });
                self.feed(Input::MediaStarted { t0 });
                self.started = None;
            }
            event @ (MediaEvent::Packet(_)
            | MediaEvent::Stats(_)
            | MediaEvent::Setup(_)
            | MediaEvent::Failed(_))
                if self.folder_start.is_some() =>
            {
                if let Some(pending) = self.folder_start.as_mut() {
                    pending.events.push(event);
                }
            }
            MediaEvent::Packet(packet) => {
                let overflowed = self
                    .recording
                    .as_mut()
                    .is_some_and(|recording| recording.push(packet));
                if overflowed {
                    self.notice(NoticeLevel::Error, DISK_TOO_SLOW);
                }
            }
            MediaEvent::Stats(stats) => {
                let queued = self.media_rx.len();
                if let Some(recording) = self.recording.as_mut() {
                    recording.on_stats(&stats, queued);
                }
            }
            MediaEvent::Setup(setup) => {
                if let Some(recording) = self.recording.as_mut() {
                    recording.on_setup(&setup);
                }
            }
            MediaEvent::Warning(text) => {
                tracing::warn!(%text, "capture warning");
                self.notice(NoticeLevel::Warn, text);
            }
            MediaEvent::Failed(text) => {
                if matches!(
                    self.session.state(),
                    SessionState::Starting { .. } | SessionState::Recording { .. }
                ) {
                    tracing::error!(%text, "capture failed");
                    if let Some(recording) = self.recording.as_mut() {
                        recording.health.on_failure();
                    }
                    let notice = if encoder::is_encoder_failure(&text) {
                        ENCODER_STOPPED.to_owned()
                    } else {
                        Problem::notice("Capture failed", &text)
                    };
                    self.notice(NoticeLevel::Error, notice);
                } else {
                    tracing::info!(%text, "capture ended after the game window closed");
                }
                self.feed(Input::MediaFailed(text));
                if self.media_session.is_some() {
                    self.stop_media();
                }
            }
        }
    }

    pub(super) fn on_internal(&mut self, msg: Internal) {
        match msg {
            Internal::MediaStopped(generation) => self.on_media_stopped(generation),
            Internal::ClipDone(result) => self.on_clip_done(result),
            Internal::MediaStart { id, result, rx } => self.on_media_start(id, result, rx),
            Internal::UnwantedStopped => self.on_unwanted_stopped(),
            Internal::DiskFree(check) => self.on_disk_free(check),
            Internal::FolderCreated(result) => self.on_folder_created(result),
            Internal::StartFailed(reason) => self.feed(Input::MediaFailed(reason)),
            Internal::WriterFailed(err) => self.on_writer_failed(&err),
            Internal::WriterDone(report) => {
                if let Some(recording) = self.recording.as_mut() {
                    recording.report = Some(report);
                }
                self.try_complete();
            }
        }
    }

    pub(super) fn start_diagnostics(&self, folder: &std::path::Path) -> Option<DiagWriter> {
        if !self.settings.recording_diagnostics {
            return None;
        }
        match DiagWriter::start(folder) {
            Ok(writer) => Some(writer),
            Err(err) => {
                tracing::warn!(%err, "cannot start the diagnostics file");
                None
            }
        }
    }

    pub(super) fn spawn_writer(
        &self,
        folder: &std::path::Path,
        started: &StartedMedia,
    ) -> Option<GameWriter> {
        if !self.settings.record_full_game {
            return None;
        }
        let tx = self.internal_tx.clone();
        let spawned = GameWriter::spawn(
            folder,
            &started.video,
            started.audio.as_ref(),
            {
                let tx = tx.clone();
                move |err| send_internal(&tx, Internal::WriterFailed(err))
            },
            move |report| {
                send_internal(&tx, Internal::WriterDone(report));
            },
        );
        match spawned {
            Ok(writer) => Some(writer),
            Err(err) => {
                tracing::error!(%err, "cannot start game writer");
                self.notice(
                    NoticeLevel::Error,
                    format!("Cannot write the game video: {err}"),
                );
                None
            }
        }
    }

    pub(super) fn try_complete(&mut self) {
        self.flush_clips();
        let Some(mut recording) = self.recording.take_if(|r| r.ready()) else {
            return;
        };
        let elapsed = hns_to_secs(mono_now() - recording.t0);
        let mut file_bytes = None;
        match recording.report.take() {
            Some(Ok(info)) => {
                recording.meta.video = VIDEO_FILE.to_owned();
                recording.meta.duration_secs = info.duration_secs;
                file_bytes = Some(info.bytes);
            }
            Some(Err(err)) => {
                tracing::error!(%err, "game video not finalized");
                if !recording.write_failed {
                    self.notice(
                        NoticeLevel::Error,
                        Problem::notice("The game video could not be saved", &err),
                    );
                }
                recording.meta.video = String::new();
                recording.meta.duration_secs = elapsed;
            }
            None => recording.meta.duration_secs = elapsed,
        }
        recording.meta.health = recording
            .health
            .has_video()
            .then(|| recording.health.finish(file_bytes));
        recording.diag.take();
        self.set_active_folder(None);
        recording.meta.game.result = recording.result.unwrap_or(GameResult::Unknown);
        let (folder, meta) = recording.snapshot(mono_now());
        if let Err(err) = self.sidecars.write_final(&folder, &meta) {
            tracing::error!(%err, "cannot write final sidecar");
            self.notice(
                NoticeLevel::Error,
                format!("Cannot save recording details: {err}"),
            );
        }
        self.test_stop_at = None;
        self.feed(Input::Finalized);
        self.send(UiMsg::RecordingSaved(RecordingEntry {
            size_bytes: library::size_bytes(&recording.folder),
            meta: recording.meta,
            folder: recording.folder,
        }));
        storage::spawn(
            self.settings.output_dir.clone(),
            self.settings.storage_limit_bytes(),
            self.active_folder.clone(),
            self.sink.clone(),
        );
    }

    pub(super) fn set_active_folder(&self, id: Option<String>) {
        if let Ok(mut active) = self.active_folder.lock() {
            *active = id;
        }
    }
}

pub(super) fn folder_id(folder: &std::path::Path) -> String {
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(super) fn send_internal(tx: &crossbeam_channel::Sender<Internal>, msg: Internal) {
    if tx.send(msg).is_err() {
        tracing::debug!("engine stopped before an internal message was delivered");
    }
}
