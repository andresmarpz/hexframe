#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod icons;
mod logging;
mod tray;
mod ui;
mod view;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use greyscreen_core::install::HIDDEN_FLAG;
use greyscreen_core::media::MediaSource;
use greyscreen_core::paths;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{Engine, league_factory_default};
use rune::{App, AppSpec};

use ui::Ui;

#[cfg(windows)]
const SINGLE_INSTANCE_ID: &str = "dev.greyscreen.app";

#[cfg(windows)]
fn media_source() -> Arc<dyn MediaSource> {
    Arc::new(greyscreen_capture::WindowsMediaSource::new())
}

#[cfg(not(windows))]
fn media_source() -> Arc<dyn MediaSource> {
    Arc::new(greyscreen_engine::FakeMediaSource::new())
}

#[cfg(windows)]
fn install_step() -> Option<greyscreen_capture::install::InstallError> {
    use greyscreen_core::install::UNINSTALL_FLAG;
    if std::env::args().any(|arg| arg == UNINSTALL_FLAG) {
        greyscreen_capture::install::uninstall();
        std::process::exit(0);
    }
    if greyscreen_capture::install::is_installed_copy()
        || std::env::var_os("GREYSCREEN_NO_INSTALL").is_some()
    {
        return None;
    }
    match greyscreen_capture::install::install_and_relaunch() {
        Ok(true) => std::process::exit(0),
        Ok(false) => None,
        Err(err) => Some(err),
    }
}

fn main() {
    let headless = rune::is_headless();
    #[cfg(windows)]
    let install_error = if headless { None } else { install_step() };
    #[cfg(windows)]
    if !headless {
        greyscreen_capture::install::set_process_app_id();
    }
    #[cfg(windows)]
    let instance = if headless {
        None
    } else {
        // Rune's single_instance makes live resize lag on Windows, so the app keeps its own.
        match greyscreen_capture::instance::claim(SINGLE_INSTANCE_ID) {
            greyscreen_capture::instance::Claim::First(signal) => Some(signal),
            greyscreen_capture::instance::Claim::AlreadyRunning => return,
        }
    };
    let _log_guard = (!headless).then(logging::init).flatten();
    tracing::info!("greyscreen {} starting", env!("GREYSCREEN_BUILD"));
    #[cfg(windows)]
    if let Some(err) = &install_error {
        tracing::warn!(%err, "cannot install, running from this folder");
    }
    #[cfg(windows)]
    greyscreen_capture::crash::install(Some(greyscreen_core::paths::logs_dir().join("crash.txt")));
    #[cfg(windows)]
    if !headless && let Err(err) = greyscreen_capture::probe::init_process() {
        tracing::warn!(%err, "cannot set process DPI awareness");
    }
    if let Err(err) = rune::set_app_icon(include_bytes!("../assets/icon.png").as_slice()) {
        tracing::warn!(%err, "cannot set app icon");
    }
    let spec = AppSpec {
        min_size: Some((720.0, 520.0)),
        scenes: ui::SCENES,
        poll: Some(Ui::poll),
        setup: Some(|_, waker| view::champion::set_waker(waker)),
        on_key: Some(Ui::on_key),
        ..AppSpec::new("Greyscreen", (960.0, 640.0), Ui::preview, view::root)
    };
    let mut app = App::new(spec).keep_running().release_gpu_when_closed();
    if std::env::args().any(|arg| arg == HIDDEN_FLAG) {
        app = app.windowless();
    }
    let mut settings = Settings::load(&paths::settings_file());
    settings.validate();
    let (_tray, _engine) = if headless {
        (None, None)
    } else {
        let handle = app.handle();
        #[cfg(windows)]
        if let Some(signal) = instance {
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
        let sink_handle = handle.clone();
        let engine = Engine::start(
            settings.clone(),
            media_source(),
            league_factory_default(),
            Box::new(move |msg| sink_handle.update(move |ui: &mut Ui, tx| ui.apply(msg, tx))),
        );
        #[cfg(windows)]
        if install_error.is_some() {
            handle.update(|ui: &mut Ui, tx| {
                ui.show_notice(
                    greyscreen_engine::NoticeLevel::Warn,
                    "Could not install, running from this folder".into(),
                    tx,
                );
            });
        }
        icons::start_prefetch(handle.clone());
        let cmd = engine.cmd.clone();
        app = app.init(move || {
            #[allow(unused_mut)]
            let mut ui = Ui::live(settings, cmd);
            #[cfg(windows)]
            {
                ui.start_with_windows = greyscreen_capture::install::start_with_windows();
            }
            ui
        });
        (tray::start(handle), Some(engine))
    };
    app.run();
}
