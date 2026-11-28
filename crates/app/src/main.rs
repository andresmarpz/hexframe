#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod champion_icons;
mod headless;
mod logging;
mod logo;
mod platform;
mod quit;
mod setup;
mod sfx;
mod tray;
mod ui;
mod view;

#[cfg(test)]
mod tests;

use greyscreen_core::encoder::PROBE_FLAG;
use greyscreen_core::install::HIDDEN_FLAG;
use greyscreen_core::paths;
use greyscreen_core::settings::NATIVE_HEIGHT_CEILING;
use greyscreen_core::settings::Settings;
use greyscreen_engine::updater::UpdateConfig;
use greyscreen_engine::{
    EncoderCheckSpec, Engine, EngineConfig, EngineStatus, FeedbackConfig, league_factory_default,
    run_probe_child, spawn_isolated_encoder_check,
};
use rune::{App, AppSpec, Chrome};

use sfx::Sfx;
use ui::Ui;

#[cfg(windows)]
#[global_allocator]
static ALLOCATOR: greyscreen_capture::crash::ReportOutOfMemory<std::alloc::System> =
    greyscreen_capture::crash::ReportOutOfMemory(std::alloc::System);

// A stuck engine join must not keep the process alive after Quit.
const ENGINE_SHUTDOWN_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
const ENCODER_CHECK_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

fn main() {
    let probe_args: Vec<String> = std::env::args().skip(1).collect();
    if probe_args.first().map(String::as_str) == Some(PROBE_FLAG) {
        platform::init_dpi();
        let code = run_probe_child(&probe_args, &*platform::media_source()).unwrap_or(2);
        std::process::exit(code);
    }
    let headless = rune::is_headless();
    if headless && setup::scene_requested() {
        return setup::run();
    }
    let Some(mut launch) = platform::begin(headless) else {
        return;
    };
    let _log_guard = (!headless).then(logging::init).flatten();
    tracing::info!("greyscreen {} starting", env!("GREYSCREEN_BUILD"));
    launch.configure_process();
    if let Err(err) = rune::set_app_icon(include_bytes!("../assets/icon.png").as_slice()) {
        tracing::warn!(%err, "cannot set app icon");
    }
    let mut settings = Settings::load(&paths::settings_file());
    settings.validate();
    let opening_size = settings
        .window_size
        .map_or((1280.0, 760.0), |(w, h)| (w as f32, h as f32));
    let spec = AppSpec {
        min_size: Some((1000.0, 600.0)),
        scenes: ui::scene_list(),
        poll: Some(Ui::poll),
        setup: Some(|_, waker| {
            view::thumb::set_waker(waker.clone());
            view::champion::set_waker(waker);
        }),
        on_key: Some(Ui::on_key),
        ..AppSpec::new("Greyscreen", opening_size, Ui::preview, view::root)
    };
    let mut app = App::new(spec)
        .chrome(Chrome {
            traffic_lights: Some((16.0, 17.0)),
            ..Chrome::DEFAULT
        })
        .maximized(settings.window_maximized)
        .on_window(ui::remember_window)
        .keep_running()
        .release_gpu_when_closed();
    if let Some(mode) = font_smoothing() {
        app = app.font_smoothing(mode);
    }
    if std::env::args().any(|arg| arg == HIDDEN_FLAG) {
        app = app.windowless();
    }
    let (tray, engine) = if headless {
        (None, None)
    } else {
        let handle = app.handle();
        launch.serve_second_launch(&handle);
        let sink_handle = handle.clone();
        let media = platform::media_source();
        let engine = Engine::start_with(EngineConfig {
            settings: settings.clone(),
            media: media.clone(),
            league: league_factory_default(),
            sink: Box::new(move |msg| sink_handle.update(move |ui: &mut Ui, tx| ui.apply(msg, tx))),
            settings_path: Some(paths::settings_file()),
            updates: Some(UpdateConfig::github(
                env!("CARGO_PKG_VERSION"),
                platform::can_install_updates(),
            )),
            feedback: Some(FeedbackConfig::api(env!("CARGO_PKG_VERSION"))),
            hotkeys: None,
            free_space: None,
            timing: None,
        });
        if launch.install_failed() {
            handle.update(|ui: &mut Ui, tx| {
                ui.show_notice(
                    greyscreen_engine::NoticeLevel::Warn,
                    "Could not install, running from this folder".into(),
                    tx,
                );
            });
        }
        let check_handle = handle.clone();
        spawn_isolated_encoder_check(
            EncoderCheckSpec {
                media,
                cfg: settings.capture_config(NATIVE_HEIGHT_CEILING),
                cache: paths::encoder_check_file(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                delay: ENCODER_CHECK_DELAY,
            },
            Box::new(move |msg| check_handle.update(move |ui: &mut Ui, tx| ui.apply(msg, tx))),
        );
        champion_icons::start_prefetch(handle.clone());
        let cmd = engine.cmd.clone();
        let tray = tray::start(
            handle,
            tray::TrayState {
                status: EngineStatus::Idle,
                auto_record: settings.auto_record,
            },
        );
        let link = tray.as_ref().map(|(link, _)| link.clone());
        app = app.init(move || {
            let mut ui = Ui::live(settings, cmd);
            ui.sfx = Sfx::rune();
            ui.start_with_windows = platform::start_with_windows();
            ui.tray = link;
            ui
        });
        (tray.map(|(_, owner)| owner), Some(engine))
    };
    rune::orderly_exit();
    let result = app.try_run();
    rune::exit_deadline(ENGINE_SHUTDOWN_DEADLINE);
    quit::finish(tray, engine, &paths::encoder_check_file());
    platform::relaunch_if_requested();
    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn font_smoothing() -> Option<rune::FontSmoothing> {
    use rune::FontSmoothing;
    match std::env::var("GREYSCREEN_FONT_SMOOTHING").ok().as_deref() {
        Some("on") => Some(FontSmoothing::On),
        Some("off") => Some(FontSmoothing::Off),
        Some("windfoil") => Some(FontSmoothing::Windfoil),
        Some("chromium") => Some(FontSmoothing::WindfoilChromium),
        _ => cfg!(target_os = "macos").then_some(FontSmoothing::Off),
    }
}
