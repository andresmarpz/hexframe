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
    FakeLeague,
    Recover,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor | Command::Record | Command::FakeLeague | Command::Recover => {
            eprintln!("not implemented yet");
            ExitCode::from(2)
        }
    }
}
