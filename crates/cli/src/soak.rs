mod args;
mod check;
mod clips;
mod drive;
mod gpu;
mod judge;
mod meter;
mod profiles;
mod rng;
mod window;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use greyscreen_core::media::MediaSource;
use greyscreen_core::problem::Problem;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    Engine, EngineCmd, EngineConfig, EngineStatus, LeagueFactory, NoticeLevel, RecordingEntry,
    UiMsg,
};
use greyscreen_league::LeagueMsg;
use serde::Serialize;

pub(crate) use args::Args;
use check::{CycleResult, Limits, Run, Sample, Verdict, evaluate};
use gpu::GpuReport;
use judge::{judge_recording, missing_recording};
use meter::Meter;
use profiles::WindowPlan;
use rng::Rng;
use window::{WindowHandle, WindowSource};

const SAMPLE_PERIOD: Duration = Duration::from_secs(1);
const SAVE_GRACE: Duration = Duration::from_secs(90);
const GAME_STOP_GRACE: Duration = Duration::from_secs(60);
const MIN_GAME_VIDEO_SECS: f64 = 1.0;

#[derive(Serialize)]
struct Report<'a> {
    profile: Option<String>,
    mode: String,
    limits: Limits,
    samples: &'a [Sample],
    cycles: &'a [CycleResult],
    engine_errors: &'a [String],
    gpu: &'a GpuReport,
    clip_saves_failed: u32,
    verdict: &'a Verdict,
}

pub(crate) fn run(args: &Args) -> ExitCode {
    match start(args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn start(args: &Args) -> Result<bool> {
    if args.list_profiles {
        println!("{}", profiles::describe());
        return Ok(true);
    }
    let args = &args.resolve()?;
    args.validate()?;
    #[cfg(windows)]
    crate::init_windows();
    let (media, window) = match args.window {
        profiles::WindowTarget::Cloaked => {
            let (media, handle) = WindowSource::wrap(default_media(), window::cloaked_factory()?);
            (media, Some(handle))
        }
        profiles::WindowTarget::League => (default_media(), None),
    };
    let verdict = run_with_window(args, media, default_league(args), window)?;
    Ok(verdict.passed)
}

#[cfg(windows)]
fn default_media() -> Arc<dyn MediaSource> {
    Arc::new(greyscreen_capture::WindowsMediaSource::new())
}

#[cfg(not(windows))]
fn default_media() -> Arc<dyn MediaSource> {
    Arc::new(greyscreen_engine::FakeMediaSource::new())
}

fn default_league(args: &Args) -> LeagueFactory {
    if args.game_minutes.is_some() {
        greyscreen_engine::league_factory_default()
    } else {
        Box::new(|_out: Sender<LeagueMsg>| Box::new(()))
    }
}

#[cfg(test)]
fn run_with(args: &Args, media: Arc<dyn MediaSource>, league: LeagueFactory) -> Result<Verdict> {
    run_with_window(args, media, league, None)
}

fn run_with_window(
    args: &Args,
    media: Arc<dyn MediaSource>,
    league: LeagueFactory,
    window: Option<WindowHandle>,
) -> Result<Verdict> {
    let game_mode = args.game_minutes.is_some();
    let output_dir = args
        .dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("greyscreen-soak"));
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("creating {}", output_dir.display()))?;
    let output_dir = std::path::absolute(&output_dir)
        .with_context(|| format!("resolving {}", output_dir.display()))?;
    if args.d3d_debug {
        gpu::enable_debug_layer();
    }
    let settings = Settings {
        output_dir: output_dir.clone(),
        auto_record: game_mode,
        recording_diagnostics: true,
        ..Settings::default()
    };
    let (tx, rx) = unbounded();
    let engine = Engine::start_with(EngineConfig {
        settings,
        media,
        league,
        sink: Box::new(move |msg| {
            tx.send(msg).ok();
        }),
        settings_path: None,
        updates: None,
        feedback: None,
        hotkeys: None,
        free_space: None,
        timing: None,
    });
    let mut soak = Soak::new(args, rx);
    soak.output_dir = output_dir;
    soak.window = window;
    match args.game_minutes {
        Some(minutes) => soak.run_games(&engine.cmd, minutes),
        None => soak.run_cycles(&engine.cmd),
    }
    drop(engine);
    soak.gpu = gpu::collect(args.d3d_debug);
    soak.discard_saved();
    soak.finish()
}

struct Soak<'a> {
    args: &'a Args,
    rx: Receiver<UiMsg>,
    meter: Meter,
    started: Instant,
    next_sample: Instant,
    samples: Vec<Sample>,
    cycles: Vec<CycleResult>,
    engine_errors: Vec<String>,
    gpu: GpuReport,
    pending_folders: Vec<PathBuf>,
    saved_folders: std::collections::HashSet<PathBuf>,
    recording: bool,
    stopped: bool,
    seed: Option<u64>,
    output_dir: PathBuf,
    clip_saves_failed: u32,
    clip_saves_skipped: u32,
    window: Option<WindowHandle>,
    window_failures: Vec<String>,
    closing_window: bool,
    close_failures_seen: u32,
}

impl<'a> Soak<'a> {
    fn new(args: &'a Args, rx: Receiver<UiMsg>) -> Self {
        let now = Instant::now();
        Self {
            args,
            rx,
            meter: Meter::new(),
            started: now,
            next_sample: now,
            samples: Vec::new(),
            cycles: Vec::new(),
            engine_errors: Vec::new(),
            gpu: GpuReport::default(),
            pending_folders: Vec::new(),
            saved_folders: std::collections::HashSet::new(),
            recording: false,
            stopped: false,
            seed: None,
            output_dir: PathBuf::new(),
            clip_saves_failed: 0,
            clip_saves_skipped: 0,
            window: None,
            window_failures: Vec::new(),
            closing_window: false,
            close_failures_seen: 0,
        }
    }

    fn send(&mut self, cmd: &Sender<EngineCmd>, msg: EngineCmd) -> bool {
        if self.stopped {
            return false;
        }
        if cmd.send(msg).is_err() {
            self.engine_stopped();
        }
        !self.stopped
    }

    fn engine_stopped(&mut self) {
        if !self.stopped {
            self.stopped = true;
            self.engine_errors.push("engine stopped".to_owned());
        }
    }

    fn checkpoint(&mut self) {
        let t = self.started.elapsed().as_secs_f64();
        let mut sample = self.meter.sample(t);
        sample.checkpoint = true;
        self.samples.push(sample);
    }

    fn run_cycles(&mut self, cmd: &Sender<EngineCmd>) {
        let mut rng = Rng::from_clock();
        if self.args.random_secs.is_some() {
            println!("random seed {}", rng.seed);
            self.seed = Some(rng.seed);
        }
        for index in 1..=self.args.cycles {
            println!("cycle {index} of {}", self.args.cycles);
            let (run_secs, gap_secs) = match self.args.random_secs {
                Some((min, max)) => (rng.between(min, max), rng.between(0, 3)),
                None => (self.args.seconds, self.args.gap_secs),
            };
            let secs = Duration::from_secs(u64::from(run_secs));
            let start = EngineCmd::StartTestRecording {
                secs: self.args.seconds,
            };
            let known = self.recording_folders();
            let mut started = self.send(cmd, start);
            let saved_before = self.cycles.len();
            if started && self.args.failed_clip_saves {
                self.fail_a_clip(cmd, &known);
            }
            if started && !self.args.failed_clip_saves {
                self.drive_window(cmd, index, &mut rng);
            }
            let saved_early = self.cycles.len() > saved_before;
            if started && !saved_early && self.args.random_secs.is_some() {
                self.pump(Instant::now() + secs, false);
                started = self.send(cmd, EngineCmd::StopRecording);
            }
            let saved =
                saved_early || (started && self.pump(Instant::now() + secs + SAVE_GRACE, true));
            if !saved {
                if !self.stopped {
                    self.cycles.push(missing_recording(index, secs));
                    cmd.send(EngineCmd::StopRecording).ok();
                }
                break;
            }
            let gap = Duration::from_secs(u64::from(gap_secs));
            self.pump(Instant::now() + gap, false);
            self.discard_saved();
            self.checkpoint();
            if self.stopped {
                break;
            }
        }
    }

    fn run_games(&mut self, cmd: &Sender<EngineCmd>, minutes: u32) {
        println!("waiting for games for {minutes} minutes");
        let saved = self.pump(
            self.started + Duration::from_secs(u64::from(minutes) * 60),
            self.args.until_saved,
        );
        if !saved && self.recording && self.send(cmd, EngineCmd::StopRecording) {
            self.pump(Instant::now() + GAME_STOP_GRACE, true);
        }
    }

    fn pump(&mut self, until: Instant, stop_on_saved: bool) -> bool {
        if self.stopped {
            return false;
        }
        loop {
            let now = Instant::now();
            if now >= self.next_sample {
                let t = now.duration_since(self.started).as_secs_f64();
                let mut sample = self.meter.sample(t);
                sample.recording = self.recording;
                self.samples.push(sample);
                self.next_sample = now + SAMPLE_PERIOD;
            }
            if now >= until {
                return false;
            }
            let wake = until.min(self.next_sample);
            match self.rx.recv_timeout(wake.saturating_duration_since(now)) {
                Ok(msg) => {
                    if self.on_message(msg) && stop_on_saved {
                        return true;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.engine_stopped();
                    return false;
                }
            }
        }
    }

    fn on_message(&mut self, msg: UiMsg) -> bool {
        match msg {
            UiMsg::Status(status) => {
                self.recording = matches!(status, EngineStatus::Recording { .. });
                if let EngineStatus::Error(text) = status {
                    self.engine_errors.push(text);
                }
                false
            }
            UiMsg::Notice {
                level: NoticeLevel::Error,
                text,
            }
            | UiMsg::NoticeWithAction {
                level: NoticeLevel::Error,
                text,
                ..
            } => {
                if self.closing_window && text.starts_with(Problem::LeagueNotFound.title()) {
                    self.close_failures_seen += 1;
                } else {
                    self.engine_errors.push(text);
                }
                false
            }
            UiMsg::Notice {
                level: NoticeLevel::Warn,
                text,
            } if text.starts_with(clips::CLIP_FAILED_NOTICE) => {
                self.clip_saves_failed += 1;
                false
            }
            UiMsg::Notice { text, .. } if clips::is_clip_skipped(&text) => {
                self.clip_saves_skipped += 1;
                false
            }
            UiMsg::RecordingSaved(entry) => {
                self.on_saved(&entry);
                true
            }
            _ => false,
        }
    }

    fn on_saved(&mut self, entry: &RecordingEntry) {
        let index = self.cycles.len() as u32 + 1;
        let min_secs = match self.args.game_minutes {
            Some(_) => self.args.min_video_secs.unwrap_or(MIN_GAME_VIDEO_SECS),
            None if self.args.random_secs.is_some()
                || matches!(self.args.window_plan, Some(WindowPlan::Closes { .. })) =>
            {
                0.0
            }
            None => f64::from(self.args.seconds) * 0.5,
        };
        let mut result = judge_recording(index, entry, min_secs);
        if self.close_failures_seen > 0 {
            result.capture_errors = result
                .capture_errors
                .saturating_sub(self.close_failures_seen);
            self.close_failures_seen = 0;
        }
        println!(
            "recording {index}: {:.1} s{}",
            result.video_secs,
            result
                .problem
                .as_deref()
                .map_or_else(String::new, |p| format!(", problem: {p}"))
        );
        self.saved_folders.insert(entry.folder.clone());
        self.pending_folders.push(entry.folder.clone());
        self.cycles.push(result);
    }

    fn discard_saved(&mut self) {
        for folder in self.pending_folders.drain(..) {
            if !self.args.keep && std::fs::remove_dir_all(&folder).is_err() {
                eprintln!("could not remove {}", folder.display());
            }
        }
    }

    fn finish(self) -> Result<Verdict> {
        let min_saved = match self.args.game_minutes {
            Some(_) => 1,
            None => self.args.cycles,
        };
        let mut verdict = evaluate(
            &Run {
                samples: &self.samples,
                cycles: &self.cycles,
                engine_errors: &self.engine_errors,
                min_saved,
                require_usage: cfg!(windows),
                by_checkpoint: self.args.game_minutes.is_none(),
                device_removed: self.gpu.device_removed,
            },
            &self.args.limits(),
        );
        verdict.notes.extend(self.gpu.notes());
        self.judge_extras(&mut verdict);
        verdict
            .notes
            .extend(self.seed.map(|seed| format!("random seed {seed}")));
        let out = self.args.out_path();
        let report = Report {
            profile: self.args.profile.clone(),
            mode: match self.args.game_minutes {
                Some(minutes) => format!("games {minutes} min"),
                None => format!("{} cycles of {} s", self.args.cycles, self.args.seconds),
            },
            limits: self.args.limits(),
            samples: &self.samples,
            cycles: &self.cycles,
            engine_errors: &self.engine_errors,
            gpu: &self.gpu,
            clip_saves_failed: self.clip_saves_failed,
            verdict: &verdict,
        };
        std::fs::write(&out, serde_json::to_vec_pretty(&report)?)
            .with_context(|| format!("writing {}", out.display()))?;
        print_verdict(&verdict, &out);
        Ok(verdict)
    }
}

fn print_verdict(verdict: &Verdict, out: &std::path::Path) {
    for note in &verdict.notes {
        println!("note: {note}");
    }
    if verdict.passed {
        println!("PASS: report {}", out.display());
    } else {
        for failure in &verdict.failures {
            println!("fail: {failure}");
        }
        println!("FAIL: report {}", out.display());
    }
}

#[cfg(test)]
mod tests;
