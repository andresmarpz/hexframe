mod library;
mod outcome;
mod play;
mod report;
mod run;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result, bail};
use clap::Parser;
use greyscreen_qa::Case;

use outcome::CaseOutcome;
use play::Judge;
use run::{AppRun, Sandbox};

/// Headless UI QA: runs each case in qa/cases through the real app with no window, no OS input
/// and no sound, checks the element tree and compares snapshots with the goldens.
#[derive(Parser, Debug)]
struct Args {
    /// The greyscreen binary (default: next to this one).
    #[arg(long)]
    app: Option<PathBuf>,
    /// The greyscreen-cli binary that seeds the demo library (default: next to this one).
    #[arg(long)]
    cli: Option<PathBuf>,
    #[arg(long, default_value = "qa/cases")]
    cases: PathBuf,
    /// Golden snapshots (default: qa/goldens/<os>).
    #[arg(long)]
    goldens: Option<PathBuf>,
    #[arg(long, default_value = "target/qa")]
    out: PathBuf,
    /// Write this run's snapshots as the new goldens where they differ or are missing.
    #[arg(long)]
    update: bool,
    /// A library folder to copy for library cases instead of the seeded demo library. Its videos
    /// are replaced by empty files in the copy, and its snapshots are not compared.
    #[arg(long)]
    library: Option<PathBuf>,
    /// Run only the cases whose name contains this.
    #[arg(long)]
    only: Option<String>,
    /// Largest per-channel difference a pixel may have and still match.
    #[arg(long, default_value_t = 24)]
    tolerance: u8,
    /// Largest share of changed pixels a snapshot may have and still match.
    #[arg(long, default_value_t = 0.002)]
    max_diff: f64,
    #[arg(long)]
    jobs: Option<usize>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let app = args.app.clone().unwrap_or_else(|| sibling("greyscreen"));
    let cli = args.cli.clone().unwrap_or_else(|| sibling("greyscreen-cli"));
    if !app.is_file() {
        bail!(
            "no app at {}; build it with `cargo build -p greyscreen`",
            app.display()
        );
    }
    let cases = load_cases(&args.cases, args.only.as_deref())?;
    if cases.is_empty() {
        bail!("no cases in {}", args.cases.display());
    }
    let out = absolute(&args.out)?;
    let runs = out.join("runs");
    if runs.exists() {
        std::fs::remove_dir_all(&runs)
            .with_context(|| format!("cannot clear {}", runs.display()))?;
    }
    let sandbox = Sandbox {
        tmp: out.join("tmp"),
    };
    std::fs::create_dir_all(&sandbox.tmp)?;
    let library = if cases.iter().any(|c| c.library) {
        Some(library::prepare(
            &out,
            args.library.as_deref(),
            &cli,
            &sandbox,
        )?)
    } else {
        None
    };
    let judge = Judge {
        goldens: absolute(&args.goldens.clone().unwrap_or_else(default_goldens))?,
        update: args.update,
        tolerance: args.tolerance,
        max_diff: args.max_diff,
        custom_library: args.library.is_some(),
    };
    let jobs = args
        .jobs
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get() / 2))
        .clamp(1, 6);
    let outcomes = run_all(&cases, jobs, |case| {
        let run = AppRun {
            exe: &app,
            sandbox: &sandbox,
            library: library.as_deref(),
            dir: runs.join(&case.name),
        };
        let outcome = play::play(case, &run, &judge);
        let word = if outcome.failed() { "FAIL" } else { "ok" };
        eprintln!("{word:>4} {}", case.name);
        outcome
    });
    std::fs::write(out.join("report.md"), report::markdown(&out, &outcomes))?;
    std::fs::write(out.join("report.html"), report::html(&out, &outcomes))?;
    let t = report::totals(&outcomes);
    eprintln!(
        "{} cases, {} failed, {} new snapshots. Report: {}",
        t.cases,
        t.failed,
        t.new,
        out.join("report.html").display()
    );
    if t.failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn run_all(
    cases: &[Case],
    jobs: usize,
    play: impl Fn(&Case) -> CaseOutcome + Sync,
) -> Vec<CaseOutcome> {
    let queue = Mutex::new(cases.iter().enumerate().collect::<VecDeque<_>>());
    let done = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().unwrap_or_else(|e| e.into_inner()).pop_front();
                    let Some((index, case)) = next else { break };
                    let outcome = play(case);
                    done.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push((index, outcome));
                }
            });
        }
    });
    let mut done = done.into_inner().unwrap_or_else(|e| e.into_inner());
    done.sort_by_key(|(index, _)| *index);
    done.into_iter().map(|(_, outcome)| outcome).collect()
}

fn load_cases(dir: &Path, only: Option<&str>) -> Result<Vec<Case>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("no cases folder {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let mut cases = Vec::new();
    for path in paths {
        let case = Case::load(&path)?;
        if only.is_none_or(|only| case.name.contains(only)) {
            cases.push(case);
        }
    }
    Ok(cases)
}

fn sibling(name: &str) -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    exe.with_file_name(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

fn default_goldens() -> PathBuf {
    PathBuf::from("qa/goldens").join(std::env::consts::OS)
}

fn absolute(path: &Path) -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join(path))
}
