use std::path::PathBuf;

use greyscreen_core::export::{
    EXPORTS_DIR, ExportDone, ExportEvent, ExportJob, ExportPreset, ExportRange, file_stem,
    plan_discord,
};
use greyscreen_core::library::VideoInfo;
use greyscreen_core::paths;
use greyscreen_engine::{EngineCmd, RecordingEntry};

use super::library::format::clip_title;
use super::{Ui, format_clock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Destination {
    #[default]
    Game,
    Downloads,
}

impl Destination {
    pub(crate) const ALL: [Destination; 2] = [Destination::Game, Destination::Downloads];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Destination::Game => "This game's folder",
            Destination::Downloads => "Downloads",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Handle {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CopyState {
    Idle,
    Copied,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExportPhase {
    Editing,
    Working {
        fraction: f32,
        shrinking: bool,
        cancelling: bool,
    },
    Done {
        done: ExportDone,
        wanted_start: f64,
        copy: CopyState,
    },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExportTarget {
    pub title: String,
    pub subtitle: String,
    pub champion: String,
    pub game_folder: PathBuf,
    pub source: PathBuf,
    pub source_offset_secs: f64,
    pub video: VideoInfo,
}

#[derive(Debug)]
pub(crate) struct ExportUi {
    pub open: bool,
    pub target: Option<ExportTarget>,
    pub range: ExportRange,
    pub bounds: (f64, f64),
    pub default_range: ExportRange,
    pub preset: ExportPreset,
    pub destination: Destination,
    pub phase: ExportPhase,
    pub can_reencode: bool,
    pub downloads: Option<PathBuf>,
}

impl Default for ExportUi {
    fn default() -> Self {
        Self {
            open: false,
            target: None,
            range: ExportRange::new(0.0, 0.0),
            bounds: (0.0, 0.0),
            default_range: ExportRange::new(0.0, 0.0),
            preset: ExportPreset::Original,
            destination: Destination::Game,
            phase: ExportPhase::Editing,
            can_reencode: greyscreen_engine::REENCODE_AVAILABLE,
            downloads: paths::downloads_dir(),
        }
    }
}

impl ExportUi {
    pub(crate) fn range_fractions(&self) -> (f32, f32) {
        (
            fraction_of(self.range.start, self.bounds),
            fraction_of(self.range.end, self.bounds),
        )
    }

    pub(crate) fn default_fractions(&self) -> (f32, f32) {
        (
            fraction_of(self.default_range.start, self.bounds),
            fraction_of(self.default_range.end, self.bounds),
        )
    }

    pub(crate) fn working(&self) -> bool {
        matches!(self.phase, ExportPhase::Working { .. })
    }

    pub(crate) fn editing(&self) -> bool {
        self.phase == ExportPhase::Editing
    }

    pub(crate) fn plan_problem(&self) -> Option<String> {
        if self.preset != ExportPreset::Discord {
            return None;
        }
        let video = self.target.as_ref()?.video;
        plan_discord(self.range.duration(), video)
            .err()
            .map(|err| err.to_string())
    }

    pub(crate) fn can_start(&self) -> bool {
        self.editing()
            && self.target.is_some()
            && self.range.duration() >= greyscreen_core::export::MIN_SECS
            && self.plan_problem().is_none()
    }

    pub(crate) fn destination_dir(&self) -> Option<PathBuf> {
        let target = self.target.as_ref()?;
        match (self.destination, &self.downloads) {
            (Destination::Downloads, Some(dir)) => Some(dir.clone()),
            _ => Some(target.game_folder.join(EXPORTS_DIR)),
        }
    }

    fn job(&self) -> Option<ExportJob> {
        let target = self.target.as_ref()?;
        Some(ExportJob {
            source: target.source.clone(),
            source_offset_secs: target.source_offset_secs,
            range: self.range,
            preset: self.preset,
            video: target.video,
            dest_dir: self.destination_dir()?,
            stem: file_stem(&target.champion, self.range, self.preset),
        })
    }

    pub(super) fn begin(&mut self, target: ExportTarget, range: ExportRange, bounds: (f64, f64)) {
        if self.working() {
            return;
        }
        self.open = true;
        self.target = Some(target);
        self.range = range;
        self.default_range = range;
        self.bounds = bounds;
        self.phase = ExportPhase::Editing;
    }
}

fn fraction_of(secs: f64, bounds: (f64, f64)) -> f32 {
    let span = bounds.1 - bounds.0;
    if span <= 0.0 {
        return 0.0;
    }
    ((secs - bounds.0) / span).clamp(0.0, 1.0) as f32
}

fn game_source(entry: &RecordingEntry) -> Option<PathBuf> {
    if entry.meta.video.is_empty() {
        return None;
    }
    let path = entry.folder.join(&entry.meta.video);
    path.is_file().then_some(path)
}

pub(crate) fn clip_target(
    entry: &RecordingEntry,
    index: usize,
) -> Option<(ExportTarget, ExportRange, (f64, f64))> {
    let clip = entry.meta.clips.get(index)?;
    let range = ExportRange::new(clip.start_secs, clip.end_secs);
    let champion = entry.meta.game.info.me.champion.clone();
    let (source, offset, bounds) = match game_source(entry) {
        Some(path) => (path, 0.0, range.bounds(entry.meta.duration_secs)),
        None => {
            let path = entry.folder.join(&clip.file);
            path.is_file().then_some(())?;
            (path, clip.start_secs, (range.start, range.end))
        }
    };
    let target = ExportTarget {
        title: "Export clip".into(),
        subtitle: format!("{champion} · {}", clip_title(&clip.label)),
        champion,
        game_folder: entry.folder.clone(),
        source,
        source_offset_secs: offset,
        video: entry.meta.video_format,
    };
    Some((target, range, bounds))
}

pub(crate) fn moment_target(
    entry: &RecordingEntry,
    at: f64,
) -> Option<(ExportTarget, ExportRange, (f64, f64))> {
    let source = game_source(entry)?;
    let total = entry.meta.duration_secs;
    let range = ExportRange::moment(at, total);
    let champion = entry.meta.game.info.me.champion.clone();
    let target = ExportTarget {
        title: "Export this moment".into(),
        subtitle: format!("{champion} · around {}", format_clock(at as u32)),
        champion,
        game_folder: entry.folder.clone(),
        source,
        source_offset_secs: 0.0,
        video: entry.meta.video_format,
    };
    Some((target, range, range.bounds(total)))
}

impl Ui {
    pub(crate) fn open_export_clip(&mut self, id: &str, index: usize) {
        let entry = self
            .review
            .as_ref()
            .map(|review| &review.entry)
            .filter(|entry| entry.meta.id == id)
            .or_else(|| self.recordings.iter().find(|rec| rec.meta.id == id));
        let Some((target, range, bounds)) = entry.and_then(|entry| clip_target(entry, index))
        else {
            return;
        };
        self.close_settings();
        self.export.begin(target, range, bounds);
    }

    pub(crate) fn open_export_moment(&mut self) {
        let Some(review) = self.review.as_ref().filter(|r| r.can_export_moment()) else {
            return;
        };
        let Some((target, range, bounds)) = moment_target(&review.entry, review.time()) else {
            return;
        };
        self.close_settings();
        self.export.begin(target, range, bounds);
    }

    pub(crate) fn close_export(&mut self) {
        if let ExportPhase::Working {
            cancelling: false, ..
        } = self.export.phase
        {
            self.cancel_export();
            return;
        }
        self.export.open = false;
        self.export.phase = ExportPhase::Editing;
    }

    pub(crate) fn set_export_preset(&mut self, index: usize) {
        let Some(preset) = ExportPreset::ALL.get(index).copied() else {
            return;
        };
        let export = &mut self.export;
        if export.editing() && (export.can_reencode || preset == ExportPreset::Original) {
            export.preset = preset;
        }
    }

    pub(crate) fn set_export_destination(&mut self, index: usize) {
        let Some(destination) = Destination::ALL.get(index).copied() else {
            return;
        };
        let export = &mut self.export;
        if export.editing() && (destination == Destination::Game || export.downloads.is_some()) {
            export.destination = destination;
        }
    }

    pub(crate) fn drag_export_handle(&mut self, handle: Handle, fraction: f32) {
        let export = &mut self.export;
        if !export.editing() {
            return;
        }
        let (low, high) = export.bounds;
        let secs = (low + f64::from(fraction.clamp(0.0, 1.0)) * (high - low)).round();
        export.range = match handle {
            Handle::Start => export.range.with_start(secs, export.bounds),
            Handle::End => export.range.with_end(secs, export.bounds),
        };
    }

    pub(crate) fn start_export(&mut self) {
        if !self.export.can_start() {
            return;
        }
        let Some(job) = self.export.job() else { return };
        self.export.phase = ExportPhase::Working {
            fraction: 0.0,
            shrinking: false,
            cancelling: false,
        };
        self.send(EngineCmd::Export(job));
    }

    pub(crate) fn cancel_export(&mut self) {
        if let ExportPhase::Working { cancelling, .. } = &mut self.export.phase
            && !*cancelling
        {
            *cancelling = true;
            self.send(EngineCmd::CancelExport);
        }
    }

    pub(crate) fn export_again(&mut self) {
        if matches!(self.export.phase, ExportPhase::Failed(_)) {
            self.export.phase = ExportPhase::Editing;
        }
    }

    pub(crate) fn apply_export(&mut self, event: ExportEvent) {
        let wanted_start = self.export.range.start;
        let export = &mut self.export;
        match (&mut export.phase, event) {
            (ExportPhase::Working { fraction, .. }, ExportEvent::Progress(value)) => {
                *fraction = value.clamp(0.0, 1.0);
            }
            (
                ExportPhase::Working {
                    fraction,
                    shrinking,
                    ..
                },
                ExportEvent::Shrinking,
            ) => {
                *fraction = 0.0;
                *shrinking = true;
            }
            (ExportPhase::Working { .. }, ExportEvent::Done(done)) => {
                export.phase = ExportPhase::Done {
                    done,
                    wanted_start,
                    copy: CopyState::Idle,
                };
            }
            (ExportPhase::Working { .. }, ExportEvent::Failed(text)) => {
                export.phase = ExportPhase::Failed(text);
            }
            (ExportPhase::Working { .. }, ExportEvent::Cancelled) => {
                export.open = false;
                export.phase = ExportPhase::Editing;
            }
            _ => {}
        }
    }

    pub(crate) fn export_reveal(&self) {
        if let ExportPhase::Done { done, .. } = &self.export.phase {
            crate::headless::reveal(&done.path);
        }
    }

    pub(crate) fn export_copy(&mut self) {
        if let ExportPhase::Done { done, copy, .. } = &mut self.export.phase {
            *copy = match crate::headless::copy_file(&done.path) {
                Ok(()) => CopyState::Copied,
                Err(why) => CopyState::Failed(why),
            };
        }
    }
}
