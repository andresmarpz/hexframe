mod doctor;
mod fake_league;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "greyscreen-cli", version, about = "Greyscreen console tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Doctor,
    Record,
    FakeLeague {
        #[arg(long, default_value_t = 2999)]
        port: u16,
        #[arg(long, default_value_t = 1.0)]
        speed: f64,
        #[arg(long)]
        script: Option<PathBuf>,
    },
    Recover,
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
        Command::Record | Command::Recover => {
            eprintln!("not implemented yet");
            ExitCode::from(2)
        }
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
