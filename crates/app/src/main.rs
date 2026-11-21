#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod logging;
mod tray;
mod ui;
mod view;

#[cfg(test)]
mod tests;

use greyscreen_core::paths;
use greyscreen_core::settings::Settings;
use rune::{App, AppSpec};

use ui::Ui;

const SINGLE_INSTANCE_ID: &str = "dev.greyscreen.app";

fn main() {
    let headless = rune::is_headless();
    if !headless && !rune::single_instance(SINGLE_INSTANCE_ID) {
        return;
    }
    let _log_guard = (!headless).then(logging::init).flatten();
    let spec = AppSpec {
        min_size: Some((720.0, 520.0)),
        scenes: ui::SCENES,
        ..AppSpec::new("Greyscreen", (960.0, 640.0), Ui::preview, view::root)
    };
    let mut app = App::new(spec).keep_running().release_gpu_when_closed();
    let _tray = if headless {
        None
    } else {
        app = app.init(|| {
            let mut ui = Ui::preview();
            let path = paths::settings_file();
            ui.load_settings(Settings::load(&path), path);
            ui
        });
        tray::start(app.handle())
    };
    app.run();
}
