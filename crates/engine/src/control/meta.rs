use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use greyscreen_core::game::{GameInfo, GameResult, Player, Team, TimedEvent};
use greyscreen_core::highlight::Highlight;
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_core::time::{HNS_PER_SEC, Hns};
use jiff::Zoned;

use crate::api::FeedItem;
use crate::ring::ClipRing;
use crate::writer::{GameWriter, Report};

const SIDECAR_DEBOUNCE: Hns = 5 * HNS_PER_SEC;
const MANUAL_NAME: &str = "Test";
const MANUAL_MODE: &str = "MANUAL";

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct Recording {
    pub(super) meta: RecordingMeta,
    pub(super) folder: PathBuf,
    pub(super) t0: Hns,
    pub(super) video: VideoFormat,
    pub(super) audio: Option<AudioFormat>,
    pub(super) ring: ClipRing,
    pub(super) writer: Option<GameWriter>,
    pub(super) report: Option<Report>,
    pub(super) result: Option<GameResult>,
    pub(super) media_stopped: bool,
    pub(super) dirty: bool,
    pub(super) last_write: Hns,
}

impl Recording {
    pub(super) fn push(&mut self, packet: Packet) {
        let key_video = packet.track == Track::Video && packet.key;
        let packet = Arc::new(packet);
        if let Some(writer) = &self.writer {
            writer.send(packet.clone());
        }
        self.ring.push(packet);
        if key_video {
            self.ring.prune();
        }
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
    }

    pub(super) fn write_sidecar(&mut self, now: Hns) -> Result<(), library::LibraryError> {
        self.dirty = false;
        self.last_write = now;
        library::write_meta(&self.folder, &self.meta)
    }

    pub(super) fn sidecar_due(&self, now: Hns) -> bool {
        self.dirty && now - self.last_write >= SIDECAR_DEBOUNCE
    }
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
