use std::thread;
use std::time::{Duration, Instant};

use crate::platform::install::{self, Progress};
use crossbeam_channel::{Receiver, unbounded};
use rune::{App, AppHandle};

use super::state::{Cmd, Msg, SetupState};

const MIN_INSTALL_TIME: Duration = Duration::from_millis(700);

pub(super) fn attach(app: App<SetupState>) -> App<SetupState> {
    let (cmd, commands) = unbounded();
    let handle = app.handle();
    let spawned = thread::Builder::new()
        .name("greyscreen-setup".into())
        .spawn(move || work(&commands, &handle));
    if let Err(err) = spawned {
        tracing::warn!(%err, "cannot start the setup worker");
    }
    app.init(move || {
        let path = install::install_dir().map_or_else(
            |_| String::from("your user folder"),
            |dir| dir.display().to_string(),
        );
        SetupState::new(path, Some(cmd))
    })
}

fn work(commands: &Receiver<Cmd>, handle: &AppHandle<SetupState>) {
    for cmd in commands {
        match cmd {
            Cmd::Install { start_with_windows } => {
                let msg = match run_install(start_with_windows, handle) {
                    Ok(()) => Msg::Done,
                    Err(err) => {
                        tracing::warn!(%err, "setup failed");
                        Msg::Failed(err.to_string())
                    }
                };
                send(handle, msg);
            }
            Cmd::OpenInstalled => match install::launch_installed([]) {
                Ok(()) => handle.quit(),
                Err(err) => send(handle, Msg::Failed(format!("cannot start Greyscreen: {err}"))),
            },
            Cmd::RunHere => match install::launch_here_without_installing() {
                Ok(()) => handle.quit(),
                Err(err) => send(handle, Msg::Failed(format!("cannot start Greyscreen: {err}"))),
            },
        }
    }
}

fn run_install(
    start_with_windows: bool,
    handle: &AppHandle<SetupState>,
) -> Result<(), install::InstallError> {
    let started = Instant::now();
    install::install(|step| match step {
        Progress::ClosingRunning => send(handle, Msg::Closing),
        Progress::Copying => send(handle, Msg::Copying),
    })?;
    if let Err(err) = install::set_start_with_windows(start_with_windows) {
        tracing::warn!(%err, "cannot change start with Windows");
    }
    thread::sleep(MIN_INSTALL_TIME.saturating_sub(started.elapsed()));
    Ok(())
}

fn send(handle: &AppHandle<SetupState>, msg: Msg) {
    handle.update(move |state: &mut SetupState, _| state.apply(msg));
}
