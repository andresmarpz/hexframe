#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod champion_icons;
mod logging;
mod logo;
mod platform;
mod qa;
mod setup;
mod sfx;
mod tray;
mod ui;
mod view;

#[cfg(test)]
mod tests;

use greyscreen_core::install::HIDDEN_FLAG;
use greyscreen_core::paths;
use greyscreen_core::settings::NATIVE_HEIGHT_CEILING;
use greyscreen_core::settings::Settings;
use greyscreen_engine::updater::UpdateConfig;
use greyscreen_engine::{
    EncoderCheckSpec, Engine, EngineConfig, EngineStatus, FeedbackConfig, league_factory_default,
    spawn_encoder_check,
};
use rune::{App, AppSpec, Chrome};

use sfx::Sfx;
use ui::Ui;

const ENCODER_CHECK_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

fn main() {
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
    if std::env::args().any(|arg| arg == HIDDEN_FLAG) {
        app = app.windowless();
    }
    let (_tray, _engine) = if headless {
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
        spawn_encoder_check(
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
        let link = tray.clone();
        app = app.init(move || {
            let mut ui = Ui::live(settings, cmd);
            ui.sfx = Sfx::rune();
            ui.start_with_windows = platform::start_with_windows();
            ui.tray = link;
            ui
        });
        (tray, Some(engine))
    };
    app.run();
    greyscreen_engine::abandon_encoder_check(&paths::encoder_check_file());
}
