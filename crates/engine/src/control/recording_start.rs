use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::thread;

use greyscreen_core::clips::ClipPlanner;
use greyscreen_core::game::{GameInfo, GameResult, TimedEvent};
use greyscreen_core::gamedata;
use greyscreen_core::health::HealthTracker;
use greyscreen_core::library::{GameRecord, META_VERSION, RecordingMeta, VideoInfo};
use greyscreen_core::media::MediaEvent;
use greyscreen_core::session::{Input, Status};
use greyscreen_core::time::{Hns, mono_now, secs_to_hns};
use jiff::Zoned;

use super::effects::{folder_id, ring_keep, send_internal};
use super::meta::{self, Recording};
use super::{Control, Internal, StartedMedia};
use crate::api::UiMsg;
use crate::ring::ClipRing;
use crate::writer::VIDEO_FILE;

const FOLDER_CRASHED: &str = "creating the recording folder crashed";

pub(super) struct PendingFolder {
    started: StartedMedia,
    since: Hns,
    started_at: Zoned,
    info: GameInfo,
    status: Status,
    pub(super) events: Vec<MediaEvent>,
    pub(super) highlights: Vec<TimedEvent>,
    pub(super) result: Option<GameResult>,
    pub(super) media_stopped: bool,
}

impl Control {
    pub(super) fn begin_recording(
        &mut self,
        since: Hns,
        game: Option<GameInfo>,
        status: Status,
    ) -> Result<(), String> {
        let started = self
            .started
            .take()
            .ok_or_else(|| "capture started without formats".to_owned())?;
        let started_at = Zoned::now();
        let info = game.unwrap_or_else(meta::manual_game);
        gamedata::report_unmapped_game(&info);
        let dir = self.settings.output_dir.clone();
        let (at, who) = (started_at.clone(), info.clone());
        let tx = self.internal_tx.clone();
        thread::Builder::new()
            .name("create-folder".into())
            .spawn(move || {
                let created =
                    catch_unwind(AssertUnwindSafe(|| meta::create_folder(&dir, at, &who)))
                        .unwrap_or_else(|_| Err(FOLDER_CRASHED.to_owned()));
                send_internal(&tx, Internal::FolderCreated(created));
            })
            .map_err(|err| format!("cannot start the folder thread: {err}"))?;
        self.folder_start = Some(PendingFolder {
            started,
            since,
            started_at,
            info,
            status,
            events: Vec::new(),
            highlights: Vec::new(),
            result: None,
            media_stopped: false,
        });
        Ok(())
    }

    pub(super) fn on_folder_created(&mut self, created: Result<PathBuf, String>) {
        let Some(pending) = self.folder_start.take() else {
            return;
        };
        match created {
            Ok(folder) => self.open_recording(pending, folder),
            Err(err) => {
                let err = format!("{}: {err}", greyscreen_core::problem::FOLDER_UNUSABLE);
                if pending.result.is_some() {
                    if pending.media_stopped {
                        self.feed(Input::Finalized);
                    } else {
                        self.orphan_finalize = true;
                    }
                }
                self.fail_recording_start(&err);
            }
        }
    }

    fn open_recording(&mut self, pending: PendingFolder, folder: PathBuf) {
        let PendingFolder {
            started,
            since,
            started_at,
            info,
            status,
            events,
            highlights,
            result,
            media_stopped,
        } = pending;
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
            recovered: false,
        };
        let rate = started.audio.as_ref().map_or(48_000, |a| a.sample_rate);
        let finalized = result.is_some();
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
            write_failed: false,
            result,
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
        let (folder, meta) = recording.snapshot(mono_now());
        self.sidecars.queue(folder, meta);
        self.set_active_folder(Some(folder_id(&recording.folder)));
        self.recording = Some(recording);
        if !finalized {
            self.send(UiMsg::Status(status));
        }
        for event in events {
            self.on_media(event);
        }
        for event in highlights {
            self.on_highlight(event);
        }
        if media_stopped && let Some(recording) = self.recording.as_mut() {
            recording.media_stopped = true;
            recording.close_writer();
        }
        self.try_complete();
    }
}
