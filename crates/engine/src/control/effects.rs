use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;

use crossbeam_channel::{bounded, never};
use greyscreen_core::game::{GameInfo, GameResult, TimedEvent};
use greyscreen_core::highlight::classify;
use greyscreen_core::library::{EventEntry, GameRecord, META_VERSION, RecordingMeta, VideoInfo};
use greyscreen_core::media::{CaptureTarget, MediaEvent};
use greyscreen_core::session::{Effect, Input, Status};
use greyscreen_core::settings::Settings;
use greyscreen_core::time::{Hns, hns_to_secs, mono_now, secs_to_hns};
use jiff::Zoned;

use super::meta::{self, Recording};
use super::{Control, Internal, StartedMedia};
use crate::api::{FeedItem, NoticeLevel, RecordingEntry, UiMsg};
use crate::ring::ClipRing;
use crate::writer::{GameWriter, VIDEO_FILE};

const DEFAULT_NATIVE_HEIGHT: u32 = 1080;
const MEDIA_QUEUE: usize = 1024;
const RING_MARGIN_SECS: u32 = 5;

pub(super) fn ring_keep(settings: &Settings) -> Hns {
    let secs = settings.clips.pre_secs + settings.clips.post_secs + RING_MARGIN_SECS;
    secs_to_hns(f64::from(secs))
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
        let cfg = self.settings.capture_config(DEFAULT_NATIVE_HEIGHT);
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
        let rec_secs = hns_to_secs(event.at - recording.t0);
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
                if let Some(recording) = self.recording.as_mut() {
                    recording.push(packet);
                }
            }
            MediaEvent::Warning(text) => {
                tracing::warn!(%text, "capture warning");
                self.notice(NoticeLevel::Warn, text);
            }
            MediaEvent::Failed(text) => {
                tracing::error!(%text, "capture failed");
                self.notice(NoticeLevel::Error, format!("Capture failed: {text}"));
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
                for event in rx.try_iter() {
                    if let MediaEvent::Packet(packet) = event {
                        recording.push(packet);
                    }
                }
                recording.media_stopped = true;
                recording.close_writer();
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
        };
        let rate = started.audio.as_ref().map_or(48_000, |a| a.sample_rate);
        let mut recording = Recording {
            meta,
            folder,
            t0: since,
            ring: ClipRing::new(since, ring_keep(&self.settings), rate),
            video: started.video,
            audio: started.audio,
            writer,
            report: None,
            result: None,
            media_stopped: false,
            dirty: false,
            last_write: mono_now(),
        };
        if let Err(err) = recording.write_sidecar(mono_now()) {
            tracing::warn!(%err, "cannot write recording sidecar");
        }
        self.recording = Some(recording);
        Ok(())
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

    fn try_complete(&mut self) {
        let Some(mut recording) = self.recording.take_if(|r| r.ready()) else {
            return;
        };
        let elapsed = hns_to_secs(mono_now() - recording.t0);
        match recording.report.take() {
            Some(Ok(info)) => {
                recording.meta.video = VIDEO_FILE.to_owned();
                recording.meta.duration_secs = info.duration_secs;
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
        recording.meta.game.result = recording.result.unwrap_or(GameResult::Unknown);
        if let Err(err) = recording.write_sidecar(mono_now()) {
            tracing::error!(%err, "cannot write final sidecar");
            self.notice(
                NoticeLevel::Error,
                format!("Cannot save recording details: {err}"),
            );
        }
        self.feed(Input::Finalized);
        self.send(UiMsg::RecordingSaved(RecordingEntry {
            meta: recording.meta,
            folder: recording.folder,
        }));
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
