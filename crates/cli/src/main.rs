mod doctor;
mod fake_league;
mod record;
mod recover;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "greyscreen-cli", version = env!("GREYSCREEN_BUILD"), about = "Greyscreen console tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Doctor,
    Record {
        #[arg(long)]
        seconds: f64,
        #[arg(long, default_value = "greyscreen-test.mp4")]
        out: PathBuf,
        #[arg(long)]
        window: Option<String>,
        #[arg(long, default_value_t = 60)]
        fps: u32,
        #[arg(long, default_value_t = 1080)]
        height: u32,
        #[arg(long)]
        no_audio: bool,
        #[arg(long)]
        cursor: bool,
    },
    FakeLeague {
        #[arg(long, default_value_t = 2999)]
        port: u16,
        #[arg(long, default_value_t = 1.0)]
        speed: f64,
        #[arg(long)]
        script: Option<PathBuf>,
    },
    Recover {
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::FakeLeague {
            port,
            speed,
            script,
        } => finish(run_fake_league(port, speed, script)),
        Command::Doctor => doctor::run(),
        Command::Record {
            seconds,
            out,
            window,
            fps,
            height,
            no_audio,
            cursor,
        } => record::run(&record::Args {
            seconds,
            out,
            window,
            fps,
            height,
            audio: !no_audio,
            cursor,
        }),
        Command::Recover { path } => finish(recover::run(&path)),
    }
}

fn run_fake_league(port: u16, speed: f64, script: Option<PathBuf>) -> anyhow::Result<()> {
    let script = match script {
        Some(path) => fake_league::Script::load(&path)?,
        None => fake_league::Script::builtin()?,
    };
    fake_league::run(port, speed, script)
}

fn finish(result: anyhow::Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
pub(crate) fn init_windows() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_env_filter(filter)
        .init();
    greyscreen_capture::crash::install(Some(greyscreen_core::paths::logs_dir().join("crash.txt")));
    if let Err(err) = greyscreen_capture::probe::init_process() {
        eprintln!("dpi awareness: {err}");
    }
}
