#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod logging;
mod tray;
mod ui;
mod view;

#[cfg(test)]
mod tests;

use std::sync::Arc;

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

fn main() {
    let headless = rune::is_headless();
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
        on_key: Some(Ui::on_key),
        ..AppSpec::new("Greyscreen", (960.0, 640.0), Ui::preview, view::root)
    };
    let mut app = App::new(spec).keep_running().release_gpu_when_closed();
    let mut settings = Settings::load(&paths::settings_file());
    settings.validate();
    let (_tray, _engine) = if headless {
        (None, None)
    } else {
        let handle = app.handle();
        #[cfg(windows)]
        if let Some(signal) = instance {
            let shown = handle.clone();
            signal.on_signal(move || {
                shown.show();
                shown.focus();
            });
        }
        let sink_handle = handle.clone();
        let engine = Engine::start(
            settings.clone(),
            media_source(),
            league_factory_default(),
            Box::new(move |msg| sink_handle.update(move |ui: &mut Ui, tx| ui.apply(msg, tx))),
        );
        let cmd = engine.cmd.clone();
        app = app.init(move || Ui::live(settings, cmd));
        (tray::start(handle), Some(engine))
    };
    app.run();
}
