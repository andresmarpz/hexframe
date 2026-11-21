use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;

use crossbeam_channel::{bounded, never};
use greyscreen_core::clips::{ClipPlanner, DUE_DELAY, MAX_PLAN};
use greyscreen_core::game::{GameInfo, GameResult, TimedEvent};
use greyscreen_core::health::HealthTracker;
use greyscreen_core::highlight::classify;
use greyscreen_core::library::{EventEntry, GameRecord, META_VERSION, RecordingMeta, VideoInfo};
use greyscreen_core::media::{CaptureTarget, MediaEvent};
use greyscreen_core::session::{Effect, Input, SessionState, Status};
use greyscreen_core::time::{Hns, hns_to_secs, mono_now, secs_to_hns};
use jiff::Zoned;

use super::meta::{self, Recording};
use super::{Control, Internal, StartedMedia};
use crate::api::{FeedItem, NoticeLevel, RecordingEntry, UiMsg};
use crate::diagnostics::DiagWriter;
use crate::library;
use crate::ring::ClipRing;
use crate::storage;
use crate::writer::{GameWriter, VIDEO_FILE};

const NATIVE_HEIGHT_CEILING: u32 = 2160;
const MEDIA_QUEUE: usize = 1024;
const RING_MARGIN_SECS: u32 = 5;
const DISK_TOO_SLOW: &str = "Disk too slow: the full game recording stopped";

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
        match self.begin_recording(since, game) {
            Ok(()) => self.send(UiMsg::Status(status)),
            Err(err) => {
                tracing::error!(%err, "cannot begin recording");
                self.notice(NoticeLevel::Error, format!("Cannot start recording: {err}"));
                self.feed(Input::MediaFailed(err));
            }
        }
    }

    fn start_media(&mut self, target: CaptureTarget) {
        let cfg = self.settings.capture_config(NATIVE_HEIGHT_CEILING);
        let (tx, rx) = bounded(MEDIA_QUEUE);
        match self.media.start(target, cfg, tx) {
            Ok(session) => {
                self.media_gen += 1;
                self.media_session = Some(session);
                self.media_rx = rx;
                self.stopping = false;
            }
            Err(err) => {
                tracing::error!(%err, "media start failed");
                self.notice(
                    NoticeLevel::Error,
                    format!("Capture failed to start: {err}"),
                );
                self.feed(Input::MediaFailed(err));
            }
        }
    }

    fn stop_media(&mut self) {
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
            None => self.orphan_finalize = true,
        }
    }

    fn on_highlight(&mut self, event: TimedEvent) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let rec_secs = recording.file_secs(event.at);
        let highlights = classify(&event.kind, &recording.meta.game.info);
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
                if let Some(recording) = self.recording.as_ref() {
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
                    self.notice(NoticeLevel::Error, format!("Capture failed: {text}"));
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
            Internal::WriterDone(report) => {
                if let Some(recording) = self.recording.as_mut() {
                    recording.report = Some(report);
                }
                self.try_complete();
            }
        }
    }

    fn on_media_stopped(&mut self, generation: u64) {
        if generation != self.media_gen {
            return;
        }
        self.stopping = false;
        let rx = std::mem::replace(&mut self.media_rx, never());
        match self.recording.as_mut() {
            Some(recording) => {
                let mut overflowed = false;
                for event in rx.try_iter() {
                    if let MediaEvent::Packet(packet) = event {
                        overflowed |= recording.push(packet);
                    }
                }
                recording.media_stopped = true;
                recording.close_writer();
                if overflowed {
                    self.notice(NoticeLevel::Error, DISK_TOO_SLOW);
                }
            }
            None if self.orphan_finalize => {
                self.orphan_finalize = false;
                self.feed(Input::Finalized);
            }
            None => {}
        }
        self.try_complete();
    }

    fn begin_recording(&mut self, since: Hns, game: Option<GameInfo>) -> Result<(), String> {
        let started = self
            .started
            .take()
            .ok_or_else(|| "capture started without formats".to_owned())?;
        let started_at = Zoned::now();
        let info = game.unwrap_or_else(meta::manual_game);
        let folder = meta::create_folder(&self.settings.output_dir, started_at.clone(), &info)?;
        let writer = self.spawn_writer(&folder, &started);
        let health = HealthTracker::new(
            since,
            &started.video,
            &started.encoder,
            started.audio.as_ref().map(|a| a.sample_rate),
        );
        let diag = self.start_diagnostics(&folder);
        let meta = RecordingMeta {
            version: META_VERSION,
            id: folder_id(&folder),
            started_at,
            duration_secs: 0.0,
            video: if writer.is_some() { VIDEO_FILE } else { "" }.to_owned(),
            game: GameRecord {
                info,
                result: GameResult::Unknown,
            },
            events: Vec::new(),
            clips: Vec::new(),
            encoder: started.encoder,
            video_format: VideoInfo {
                width: started.video.width,
                height: started.video.height,
                fps: started.video.fps,
            },
            review: Default::default(),
            health: None,
        };
        let rate = started.audio.as_ref().map_or(48_000, |a| a.sample_rate);
        let mut recording = Recording {
            meta,
            folder,
            t0: since,
            media_offset_secs: None,
            ring: ClipRing::new(since, ring_keep(), rate),
            video: started.video,
            audio: started.audio,
            writer,
            report: None,
            result: None,
            media_stopped: false,
            dirty: false,
            last_write: mono_now(),
            planner: self.settings.clips.enabled.then(|| {
                ClipPlanner::new(
                    secs_to_hns(f64::from(self.settings.clips.pre_secs)),
                    secs_to_hns(f64::from(self.settings.clips.post_secs)),
                )
            }),
            queued: Vec::new(),
            pending_clips: 0,
            clips_flushed: false,
            clip_names: Default::default(),
            health,
            diag,
        };
        if let Err(err) = recording.write_sidecar(mono_now()) {
            tracing::warn!(%err, "cannot write recording sidecar");
        }
        self.set_active_folder(Some(folder_id(&recording.folder)));
        self.recording = Some(recording);
        Ok(())
    }

    fn start_diagnostics(&self, folder: &std::path::Path) -> Option<DiagWriter> {
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

    fn spawn_writer(&self, folder: &std::path::Path, started: &StartedMedia) -> Option<GameWriter> {
        if !self.settings.record_full_game {
            return None;
        }
        let tx = self.internal_tx.clone();
        let spawned = GameWriter::spawn(
            folder,
            &started.video,
            started.audio.as_ref(),
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
                self.notice(
                    NoticeLevel::Error,
                    format!("The game video could not be saved: {err}"),
                );
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
        if let Err(err) = recording.write_sidecar(mono_now()) {
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

    fn set_active_folder(&self, id: Option<String>) {
        if let Ok(mut active) = self.active_folder.lock() {
            *active = id;
        }
    }
}

fn folder_id(folder: &std::path::Path) -> String {
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn send_internal(tx: &crossbeam_channel::Sender<Internal>, msg: Internal) {
    if tx.send(msg).is_err() {
        tracing::debug!("engine stopped before an internal message was delivered");
    }
}
