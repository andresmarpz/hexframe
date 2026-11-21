use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use greyscreen_capture::install::{self, InstallError};
use greyscreen_capture::instance::{self, Claim, InstanceSignal};
use greyscreen_core::install::{StartPlan, UNINSTALL_FLAG};
use greyscreen_core::media::MediaSource;
use greyscreen_core::paths;
use rune::AppHandle;

use crate::logging::video_log_requested;
use crate::ui::Ui;

const SINGLE_INSTANCE_ID: &str = "dev.greyscreen.app";

pub(crate) const TRASH_NAME: &str = "Recycle Bin";
pub(crate) const CAN_START_WITH_WINDOWS: bool = true;

pub(crate) struct Launch {
    headless: bool,
    video_log: Option<io::Result<PathBuf>>,
    install_error: Option<InstallError>,
    instance: Option<InstanceSignal>,
}

pub(crate) fn begin(headless: bool) -> Option<Launch> {
    if headless {
        return Some(Launch {
            headless,
            video_log: None,
            install_error: None,
            instance: None,
        });
    }
    let video_log = video_log_requested().then(send_rune_video_log);
    let install_error = install_step();
    install::set_process_app_id();
    // Rune's single_instance makes live resize lag on Windows, so the app keeps its own.
    match instance::claim(SINGLE_INSTANCE_ID) {
        Claim::First(signal) => Some(Launch {
            headless,
            video_log,
            install_error,
            instance: Some(signal),
        }),
        Claim::AlreadyRunning => None,
    }
}

fn send_rune_video_log() -> io::Result<PathBuf> {
    let path = paths::logs_dir().join("rune-video.log");
    greyscreen_capture::video_log::send_rune_video_log_to(&path)?;
    Ok(path)
}

fn install_step() -> Option<InstallError> {
    if std::env::args().any(|arg| arg == UNINSTALL_FLAG) {
        install::uninstall();
        std::process::exit(0);
    }
    match install::current_start_plan() {
        StartPlan::RunInPlace => None,
        StartPlan::ShowSetup => {
            crate::setup::run();
            std::process::exit(0)
        }
        StartPlan::InstallSilently => match install::install_and_relaunch() {
            Ok(true) => std::process::exit(0),
            Ok(false) => None,
            Err(err) => Some(err),
        },
    }
}

impl Launch {
    pub(crate) fn configure_process(&self) {
        match &self.video_log {
            Some(Ok(path)) => tracing::info!(path = %path.display(), "Rune's video log is on"),
            Some(Err(err)) => tracing::warn!(%err, "cannot open Rune's video log"),
            None => {}
        }
        if let Some(err) = &self.install_error {
            tracing::warn!(%err, "cannot install, running from this folder");
        }
        greyscreen_capture::crash::install(Some(paths::logs_dir().join("crash.txt")));
        if !self.headless {
            init_dpi();
        }
    }

    pub(crate) fn install_failed(&self) -> bool {
        self.install_error.is_some()
    }

    pub(crate) fn serve_second_launch(&mut self, handle: &AppHandle<Ui>) {
        let Some(signal) = self.instance.take() else {
            return;
        };
        let shown = handle.clone();
        let quitting = handle.clone();
        signal.on_signal(
            move || {
                shown.show();
                shown.focus();
            },
            move || {
                tracing::info!("quit requested by another copy");
                quitting.quit();
            },
        );
    }
}

pub(crate) fn init_dpi() {
    if let Err(err) = greyscreen_capture::probe::init_process() {
        tracing::warn!(%err, "cannot set process DPI awareness");
    }
}

pub(crate) fn media_source() -> Arc<dyn MediaSource> {
    Arc::new(greyscreen_capture::WindowsMediaSource::new())
}

pub(crate) fn can_install_updates() -> bool {
    install::is_installed_copy()
}

pub(crate) fn connected_display_sizes() -> Vec<(u32, u32)> {
    greyscreen_capture::displays::connected_sizes()
}

pub(crate) fn start_with_windows() -> bool {
    install::start_with_windows()
}

pub(crate) fn set_start_with_windows(enabled: bool) {
    std::thread::spawn(move || {
        if let Err(err) = install::set_start_with_windows(enabled) {
            tracing::warn!(%err, "cannot change start with Windows");
        }
    });
}
