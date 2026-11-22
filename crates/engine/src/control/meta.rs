use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use greyscreen_core::clips::{ClipPlan, ClipPlanner};
use greyscreen_core::game::{GameInfo, GameResult, Player, Team, TimedEvent};
use greyscreen_core::health::{CaptureSetup, CaptureStats, HealthTracker, Queues};
use greyscreen_core::highlight::Highlight;
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_core::time::{HNS_PER_SEC, Hns, hns_to_secs};
use jiff::Zoned;

use crate::api::FeedItem;
use crate::diagnostics::DiagWriter;
use crate::ring::ClipRing;
use crate::writer::{GameWriter, Report, Sent};

const SIDECAR_DEBOUNCE: Hns = 5 * HNS_PER_SEC;
const MANUAL_NAME: &str = "Test";
const MANUAL_MODE: &str = "MANUAL";

#[derive(Debug)]
pub(super) struct Recording {
    pub(super) meta: RecordingMeta,
    pub(super) folder: PathBuf,
    pub(super) t0: Hns,
    pub(super) media_offset_secs: Option<f64>,
    pub(super) video: VideoFormat,
    pub(super) audio: Option<AudioFormat>,
    pub(super) ring: ClipRing,
    pub(super) writer: Option<GameWriter>,
    pub(super) report: Option<Report>,
    pub(super) write_failed: bool,
    pub(super) result: Option<GameResult>,
    pub(super) media_stopped: bool,
    pub(super) dirty: bool,
    pub(super) last_write: Hns,
    pub(super) planner: Option<ClipPlanner>,
    pub(super) queued: Vec<ClipPlan>,
    pub(super) pending_clips: usize,
    pub(super) clips_flushed: bool,
    pub(super) clip_names: HashSet<String>,
    pub(super) health: HealthTracker,
    pub(super) diag: Option<DiagWriter>,
}

impl Recording {
    pub(super) fn push(&mut self, packet: Packet) -> bool {
        if packet.track == Track::Video && self.media_offset_secs.is_none() {
            self.set_media_offset(video_dts_secs(&packet));
        }
        self.health.on_packet(&packet);
        let key_video = packet.track == Track::Video && packet.key;
        let packet = Arc::new(packet);
        let overflowed = self
            .writer
            .as_mut()
            .is_some_and(|writer| writer.send(packet.clone()) == Sent::Full);
        self.ring.push(packet);
        if key_video {
            self.ring.prune();
        }
        overflowed
    }

    pub(super) fn on_stats(&mut self, stats: &CaptureStats, media_queue: usize) {
        let queues = Queues {
            writer: self.writer.as_ref().map_or(0, GameWriter::queued),
            media: u32::try_from(media_queue).unwrap_or(u32::MAX),
        };
        let sample = self.health.on_stats(stats, queues);
        if let Some(diag) = &self.diag {
            diag.line("second", &sample);
        }
    }

    pub(super) fn on_setup(&mut self, setup: &CaptureSetup) {
        self.health.add_warnings(&setup.warnings);
        if let Some(diag) = &self.diag {
            diag.line("setup", setup);
        }
    }

    fn set_media_offset(&mut self, offset: f64) {
        self.media_offset_secs = Some(offset);
        for event in &mut self.meta.events {
            event.rec_secs -= offset;
        }
        self.meta.events.retain(|event| event.rec_secs >= 0.0);
        self.dirty = true;
    }

    pub(super) fn file_secs(&self, at: Hns) -> f64 {
        file_secs(at, self.t0, self.media_offset_secs.unwrap_or(0.0))
    }

    pub(super) fn close_writer(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            writer.close();
        }
    }

    pub(super) fn ready(&self) -> bool {
        self.result.is_some()
            && self.media_stopped
            && (self.writer.is_none() || self.report.is_some())
            && self.clips_flushed
            && self.pending_clips == 0
    }

    pub(super) fn snapshot(&mut self, now: Hns) -> (PathBuf, RecordingMeta) {
        self.dirty = false;
        self.last_write = now;
        (self.folder.clone(), self.meta.clone())
    }

    pub(super) fn sidecar_due(&self, now: Hns) -> bool {
        self.dirty && now - self.last_write >= SIDECAR_DEBOUNCE
    }
}

fn video_dts_secs(packet: &Packet) -> f64 {
    packet.dts as f64 / f64::from(VideoFormat::TIMESCALE)
}

pub(super) fn file_secs(at: Hns, t0: Hns, offset_secs: f64) -> f64 {
    hns_to_secs(at - t0) - offset_secs
}

pub(super) fn manual_game() -> GameInfo {
    GameInfo {
        mode: MANUAL_MODE.to_owned(),
        map: None,
        me: Player {
            name: MANUAL_NAME.to_owned(),
            champion: MANUAL_NAME.to_owned(),
            team: Team::Order,
            is_bot: false,
        },
        players: Vec::new(),
    }
}

pub(super) fn create_folder(
    output_dir: &Path,
    started: Zoned,
    info: &GameInfo,
) -> Result<PathBuf, String> {
    let base = library::folder_name(started, &info.me.champion, &info.mode);
    for attempt in 1u32..100 {
        let name = if attempt == 1 {
            base.clone()
        } else {
            format!("{base}_{attempt}")
        };
        let folder = output_dir.join(name);
        if folder.exists() {
            continue;
        }
        fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        return Ok(folder);
    }
    Err(format!("no free folder name for {base}"))
}

pub(super) fn event_name(event: &TimedEvent) -> String {
    serde_json::to_value(&event.kind)
        .ok()
        .and_then(|value| value.get("type")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| "Event".to_owned())
}

pub(super) fn feed_item(event: &TimedEvent, rec_secs: f64, highlights: Vec<Highlight>) -> FeedItem {
    let text = if highlights.is_empty() {
        event_name(event)
    } else {
        highlights
            .iter()
            .map(Highlight::label)
            .collect::<Vec<_>>()
            .join(", ")
    };
    FeedItem {
        id: event.id,
        rec_secs,
        game_secs: event.game_secs,
        text,
        highlights,
    }
}

#[cfg(test)]
mod tests;
