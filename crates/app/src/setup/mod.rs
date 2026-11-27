#[cfg(windows)]
mod driver;
mod scenes;
mod state;
mod view;

use rune::{App, AppSpec, Chrome, WindowOptions};

const WINDOW: (f32, f32) = (460.0, 420.0);

pub(crate) fn scene_requested() -> bool {
    let mut args = std::env::args();
    args.any(|arg| arg == "--scene") && args.next().is_some_and(|name| name.starts_with("setup"))
}

pub(crate) fn run() {
    let headless = rune::is_headless();
    if !headless {
        crate::platform::init_dpi();
    }
    let _log_guard = (!headless).then(crate::logging::init).flatten();
    if let Err(err) = rune::set_app_icon(include_bytes!("../../assets/icon.png").as_slice()) {
        tracing::warn!(%err, "cannot set app icon");
    }
    let spec = AppSpec {
        min_size: Some(WINDOW),
        scenes: scenes::SCENES,
        ..AppSpec::new("Greyscreen Setup", WINDOW, scenes::preview, view::root)
    };
    let app = App::new(spec)
        .chrome(Chrome {
            full_size_content: false,
            ..Chrome::DEFAULT
        })
        .window_options(WindowOptions {
            resizable: false,
            max_size: Some(WINDOW),
            ..WindowOptions::default()
        });
    #[cfg(windows)]
    let app = driver::attach(app);
    app.run();
}
