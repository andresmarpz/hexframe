mod media;
mod script;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use greyscreen_core::clips::ClipPlanner;
use greyscreen_core::highlight::classify;
use greyscreen_core::library::{
    self, ClipEntry, EventEntry, GameRecord, META_FILE, META_VERSION, RecordingMeta, VideoInfo,
};
use greyscreen_core::paths;
use greyscreen_core::settings::{ClipSettings, Settings};
use greyscreen_core::time::{hns_to_secs, secs_to_hns};
use greyscreen_mp4::clip::write_clip;
use greyscreen_mp4::fragmented::FragmentedWriter;
use jiff::{Span, Zoned};

use media::Media;
use script::Spec;

const DEMO_ENCODER: &str = "demo";
const VIDEO_FILE: &str = "game.mp4";
const CLIPS_DIR: &str = "clips";

#[derive(Subcommand, Debug)]
pub(crate) enum Action {
    Seed {
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long, default_value_t = 6)]
        games: usize,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        now: Option<Zoned>,
    },
    Clear {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

pub(crate) fn run(action: Action) -> Result<()> {
    match action {
        Action::Seed {
            dir,
            games,
            force,
            now,
        } => seed(&resolve(dir), games, force, now.unwrap_or_else(Zoned::now)),
        Action::Clear { dir } => clear(&resolve(dir)),
    }
}

fn resolve(dir: Option<PathBuf>) -> PathBuf {
    dir.unwrap_or_else(|| Settings::load(&paths::settings_file()).output_dir)
}

fn seed(dir: &Path, games: usize, force: bool, now: Zoned) -> Result<()> {
    if games == 0 {
        bail!("--games must be at least 1");
    }
    let planned: Vec<(Spec, Zoned, PathBuf)> = script::specs(games)
        .into_iter()
        .map(|spec| {
            let started = now.checked_sub(Span::new().hours(spec.hours_ago))?;
            let name = library::folder_name(started.clone(), spec.champion, spec.mode);
            Ok((spec, started, dir.join(name)))
        })
        .collect::<Result<_>>()?;
    for (_, _, folder) in &planned {
        if folder.exists() && !force {
            bail!(
                "{} already exists, pass --force to replace it",
                folder.display()
            );
        }
    }
    let longest = planned
        .iter()
        .map(|(spec, _, _)| spec.media_secs)
        .fold(0.0, f64::max);
    let media = Media::collect(longest + 1.0)?;
    for (spec, started, folder) in planned {
        if folder.exists() {
            fs::remove_dir_all(&folder)
                .with_context(|| format!("removing {}", folder.display()))?;
        }
        write_game(&media, &spec, started, &folder)?;
        println!("{}", folder.display());
    }
    println!("Open Greyscreen to see the demo games. Remove them with: greyscreen-cli demo clear");
    Ok(())
}

fn write_game(media: &Media, spec: &Spec, started_at: Zoned, folder: &Path) -> Result<()> {
    fs::create_dir_all(folder.join(CLIPS_DIR))?;
    let info = script::game_info(spec);
    let video_path = folder.join(VIDEO_FILE);
    let mut writer = FragmentedWriter::create(&video_path, &media.video, Some(&media.audio))?;
    for packet in media.upto(spec.media_secs) {
        writer.push(&packet)?;
    }
    let finished = writer.finish()?;

    let settings = ClipSettings::default();
    let mut planner = ClipPlanner::new(
        secs_to_hns(f64::from(settings.pre_secs)),
        secs_to_hns(f64::from(settings.post_secs)),
    );
    let loading = script::loading_secs();
    let mut events = Vec::new();
    for (index, (game_secs, kind)) in script::timeline(spec).into_iter().enumerate() {
        let id = u32::try_from(index)?;
        let rec_secs = game_secs + loading;
        let highlights = classify(&kind, &info);
        if let Some(worthy) = highlights.iter().find(|h| h.is_clip_worthy(&settings)) {
            planner.on_highlight(secs_to_hns(rec_secs), worthy.label(), id);
        }
        events.push(EventEntry {
            id,
            game_secs,
            rec_secs,
            kind,
            highlights: highlights.iter().map(|h| h.label().to_owned()).collect(),
        });
    }

    let mut clips = Vec::new();
    let mut taken = HashSet::new();
    for plan in planner.flush() {
        let start = hns_to_secs(plan.start).max(0.0);
        let end = hns_to_secs(plan.end);
        let moment = hns_to_secs(plan.moment);
        let name = unique_name(&library::clip_file_name(moment, &plan.label), &taken);
        let file = format!("{CLIPS_DIR}/{name}");
        let packets = media.clip(start, end);
        write_clip(
            &folder.join(&file),
            &media.video,
            Some(&media.audio),
            &packets,
        )
        .with_context(|| format!("writing {file}"))?;
        taken.insert(name);
        clips.push(ClipEntry {
            file,
            start_secs: start,
            end_secs: end,
            label: plan.label,
        });
    }

    let meta = RecordingMeta {
        version: META_VERSION,
        id: folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        started_at,
        duration_secs: finished.duration_secs,
        video: VIDEO_FILE.to_owned(),
        game: GameRecord {
            info,
            result: spec.result,
        },
        events,
        clips,
        encoder: DEMO_ENCODER.to_owned(),
        video_format: VideoInfo {
            width: media.video.width,
            height: media.video.height,
            fps: media.video.fps,
        },
        review: Default::default(),
    };
    library::write_meta(folder, &meta)?;
    Ok(())
}

fn unique_name(base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_owned();
    }
    let stem = base.strip_suffix(".mp4").unwrap_or(base);
    (2u32..)
        .map(|n| format!("{stem}_{n}.mp4"))
        .find(|candidate| !taken.contains(candidate))
        .unwrap_or_else(|| base.to_owned())
}

fn clear(dir: &Path) -> Result<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        println!("nothing to remove in {}", dir.display());
        return Ok(());
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let folder = entry.path();
        if !folder.join(META_FILE).is_file() {
            continue;
        }
        let Ok(meta) = library::read_meta(&folder) else {
            continue;
        };
        if meta.encoder != DEMO_ENCODER {
            continue;
        }
        fs::remove_dir_all(&folder).with_context(|| format!("removing {}", folder.display()))?;
        println!("removed {}", folder.display());
        removed += 1;
    }
    println!("removed {removed} demo games");
    Ok(())
}
