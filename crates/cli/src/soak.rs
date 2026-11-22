mod check;
mod gpu;
mod meter;
mod profiles;
mod rng;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use greyscreen_core::media::MediaSource;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    Engine, EngineCmd, EngineConfig, EngineStatus, LeagueFactory, NoticeLevel, RecordingEntry,
    UiMsg,
};
use greyscreen_league::LeagueMsg;
use greyscreen_mp4::demux::Movie;
use serde::Serialize;

use check::{AvLimits, CycleResult, Limits, Run, Sample, Verdict, evaluate};
use gpu::GpuReport;
use meter::Meter;
use profiles::WindowTarget;
use rng::Rng;

const SAMPLE_PERIOD: Duration = Duration::from_secs(1);
const SAVE_GRACE: Duration = Duration::from_secs(90);
const GAME_STOP_GRACE: Duration = Duration::from_secs(60);
const MIN_GAME_VIDEO_SECS: f64 = 1.0;

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct Args {
    #[arg(
        long,
        conflicts_with_all = ["cycles", "seconds", "gap_secs", "game_minutes", "warmup_secs", "warmup_cycles", "max_mem_mb_per_min", "max_gpu_mb_per_min", "max_handle_growth", "max_thread_growth"]
    )]
    pub(crate) profile: Option<String>,
    #[arg(long)]
    pub(crate) list_profiles: bool,
    #[arg(long, value_enum, default_value = "league")]
    pub(crate) window: WindowTarget,
    #[arg(long, default_value_t = 8)]
    pub(crate) cycles: u32,
    #[arg(long, default_value_t = 30)]
    pub(crate) seconds: u32,
    #[arg(long, default_value_t = 10)]
    pub(crate) gap_secs: u32,
    #[arg(long, value_name = "MINUTES")]
    pub(crate) game_minutes: Option<u32>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) dir: Option<PathBuf>,
    #[arg(long)]
    pub(crate) keep: bool,
    #[arg(long)]
    pub(crate) d3d_debug: bool,
    #[arg(long, default_value_t = 60.0)]
    pub(crate) warmup_secs: f64,
    #[arg(long, default_value_t = 2)]
    pub(crate) warmup_cycles: u32,
    #[arg(long, default_value_t = 2.0, value_name = "MB")]
    pub(crate) max_mem_mb_per_min: f64,
    #[arg(long, default_value_t = 2.0, value_name = "MB")]
    pub(crate) max_gpu_mb_per_min: f64,
    #[arg(long, default_value_t = 50)]
    pub(crate) max_handle_growth: u32,
    #[arg(long, default_value_t = 4)]
    pub(crate) max_thread_growth: u32,
    #[arg(skip)]
    pub(crate) max_dropped_pct: Option<f64>,
    #[arg(skip)]
    pub(crate) encoder_contains: Option<&'static str>,
    #[arg(skip)]
    pub(crate) min_video_secs: Option<f64>,
    #[arg(skip)]
    pub(crate) until_saved: bool,
    #[arg(skip)]
    pub(crate) av_limits: Option<AvLimits>,
    #[arg(skip)]
    pub(crate) random_secs: Option<(u32, u32)>,
}

impl Args {
    fn out_path(&self) -> PathBuf {
        self.out
            .clone()
            .unwrap_or_else(|| PathBuf::from("greyscreen-soak.json"))
    }

    fn limits(&self) -> Limits {
        Limits {
            mem_mb_per_min: self.max_mem_mb_per_min,
            gpu_mb_per_min: self.max_gpu_mb_per_min,
            handle_growth: self.max_handle_growth,
            thread_growth: self.max_thread_growth,
            warmup_secs: self.warmup_secs,
            warmup_cycles: self.warmup_cycles,
            max_dropped_pct: self.max_dropped_pct,
            encoder_contains: self.encoder_contains,
            av: self.av_limits,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.game_minutes.is_none() && (self.cycles == 0 || self.seconds == 0) {
            bail!("--cycles and --seconds must be at least 1");
        }
        if self.game_minutes == Some(0) {
            bail!("--game-minutes must be at least 1");
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct Report<'a> {
    profile: Option<String>,
    mode: String,
    limits: Limits,
    samples: &'a [Sample],
    cycles: &'a [CycleResult],
    engine_errors: &'a [String],
    gpu: &'a GpuReport,
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
    let verdict = run_with(args, default_media(), default_league(args))?;
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

fn run_with(args: &Args, media: Arc<dyn MediaSource>, league: LeagueFactory) -> Result<Verdict> {
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
        output_dir,
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
    recording: bool,
    stopped: bool,
    seed: Option<u64>,
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
            recording: false,
            stopped: false,
            seed: None,
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
            let mut started = self.send(cmd, start);
            if started && self.args.random_secs.is_some() {
                self.pump(Instant::now() + secs, false);
                started = self.send(cmd, EngineCmd::StopRecording);
            }
            if !started || !self.pump(Instant::now() + secs + SAVE_GRACE, true) {
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
                self.engine_errors.push(text);
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
            None if self.args.random_secs.is_some() => 0.0,
            None => f64::from(self.args.seconds) * 0.5,
        };
        let result = judge_recording(index, entry, min_secs);
        println!(
            "recording {index}: {:.1} s{}",
            result.video_secs,
            result
                .problem
                .as_deref()
                .map_or_else(String::new, |p| format!(", problem: {p}"))
        );
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

fn missing_recording(index: u32, waited: Duration) -> CycleResult {
    CycleResult {
        index,
        folder: String::new(),
        video_secs: 0.0,
        dropped_pct: 0.0,
        capture_errors: 0,
        diag_seconds: Vec::new(),
        health: None,
        problem: Some(format!(
            "no saved recording {} s after the start",
            (waited + SAVE_GRACE).as_secs()
        )),
    }
}

fn judge_recording(index: u32, entry: &RecordingEntry, min_secs: f64) -> CycleResult {
    let health = entry.meta.health.as_ref();
    let (video_secs, problem) = match playable_secs(entry, min_secs) {
        Ok(secs) => (secs, None),
        Err(problem) => (0.0, Some(problem)),
    };
    CycleResult {
        index,
        folder: entry.folder.display().to_string(),
        video_secs,
        dropped_pct: health.map_or(0.0, |h| h.dropped_pct),
        capture_errors: health.map_or(0, |h| h.capture_errors),
        problem,
        health: entry.meta.health.clone(),
        diag_seconds: diag_seconds(&entry.folder),
    }
}

fn diag_seconds(folder: &std::path::Path) -> Vec<serde_json::Value> {
    let Ok(text) = std::fs::read_to_string(folder.join("diagnostics.jsonl")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|v| v["type"] == "second")
        .collect()
}

fn playable_secs(entry: &RecordingEntry, min_secs: f64) -> Result<f64, String> {
    if entry.meta.video.is_empty() {
        return Err("the recording has no video file".to_owned());
    }
    let path = entry.folder.join(&entry.meta.video);
    let movie = Movie::open(&path).map_err(|e| format!("{} does not open: {e}", path.display()))?;
    let secs = movie.video_secs();
    if movie.video_samples.is_empty() || secs < min_secs {
        return Err(format!(
            "video is {secs:.1} s, expected at least {min_secs:.1} s"
        ));
    }
    Ok(secs)
}

#[cfg(test)]
mod tests;
