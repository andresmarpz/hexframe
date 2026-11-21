mod audio;
mod diag;
mod render;
mod verdicts;
mod video;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use greyscreen_core::health::{Level, RecordingHealth};
use greyscreen_core::library::{self, META_FILE};
use greyscreen_mp4::inspect::{Inspected, Layout, inspect};
use serde::Serialize;

use audio::AudioReport;
use diag::DiagReport;
use video::{AudioBytes, VideoReport};

pub(crate) use verdicts::Verdict;

const VIDEO_FILE: &str = "game.mp4";
const PART_FILE: &str = "game.mp4.part";
const DIAG_FILE: &str = "diagnostics.jsonl";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FileReport {
    pub path: PathBuf,
    pub bytes: u64,
    pub finalized: bool,
    pub fragments: Option<u32>,
    pub trailing_bytes: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub target_fps: u32,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Report {
    pub file: FileReport,
    pub video: VideoReport,
    pub audio: Option<AudioReport>,
    pub health: Option<RecordingHealth>,
    pub diagnostics: Option<DiagReport>,
    pub verdicts: Vec<Verdict>,
    pub overall: Level,
}

pub(crate) struct Args {
    pub(crate) target: PathBuf,
    pub(crate) json: bool,
}

pub(crate) fn run(args: &Args) -> Result<()> {
    let report = analyze_path(&args.target)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render::text(&report));
    }
    Ok(())
}

pub(crate) fn analyze_path(target: &Path) -> Result<Report> {
    let (video_path, folder) = resolve(target)?;
    let media =
        inspect(&video_path).with_context(|| format!("reading {}", video_path.display()))?;
    let health = library::read_meta(&folder)
        .ok()
        .and_then(|meta| meta.health);
    let diagnostics = diag::read(&folder.join(DIAG_FILE));
    Ok(assemble(&video_path, &media, health, diagnostics))
}

fn resolve(target: &Path) -> Result<(PathBuf, PathBuf)> {
    if target.is_dir() {
        for name in [VIDEO_FILE, PART_FILE] {
            let candidate = target.join(name);
            if candidate.is_file() {
                return Ok((candidate, target.to_path_buf()));
            }
        }
        let note = if target.join(META_FILE).is_file() {
            " (the video may have been removed by the storage limit)"
        } else {
            ""
        };
        bail!("no {VIDEO_FILE} in {}{note}", target.display());
    }
    if !target.is_file() {
        bail!("{} is not a file or folder", target.display());
    }
    let folder = target
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    Ok((target.to_path_buf(), folder))
}

pub(crate) fn assemble(
    path: &Path,
    media: &Inspected,
    health: Option<RecordingHealth>,
    diagnostics: Option<DiagReport>,
) -> Report {
    let audio_rows = &media.tables.audio;
    let rate = media.audio.as_ref().map(|a| a.sample_rate);
    let video = video::analyze(
        &media.tables.video,
        media.video.fps,
        rate.map(|sample_rate| AudioBytes {
            rows: audio_rows,
            sample_rate,
        }),
    );
    let audio = rate.map(|rate| audio::analyze(audio_rows, rate, video.duration_secs));
    let (finalized, fragments, trailing_bytes) = match media.layout {
        Layout::Finalized => (true, None, None),
        Layout::Unfinalized {
            fragments,
            trailing_bytes,
        } => (false, Some(fragments), Some(trailing_bytes)),
    };
    let file = FileReport {
        path: path.to_path_buf(),
        bytes: media.bytes,
        finalized,
        fragments,
        trailing_bytes,
        width: media.video.width,
        height: media.video.height,
        target_fps: media.video.fps,
    };
    let verdicts = verdicts::collect(
        &file,
        &video,
        audio.as_ref(),
        health.as_ref(),
        diagnostics.as_ref(),
    );
    let overall = verdicts.iter().map(|v| v.level).max().unwrap_or(Level::Ok);
    Report {
        file,
        video,
        audio,
        health,
        diagnostics,
        verdicts,
        overall,
    }
}

#[cfg(test)]
mod tests;
