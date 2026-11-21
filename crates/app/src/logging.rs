use std::panic;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const VIDEO_LOG_ENV: &str = "GREYSCREEN_VIDEO_LOG";
pub(crate) const PLAYBACK_TARGET: &str = "playback";

pub(crate) fn video_log_requested() -> bool {
    std::env::var_os(VIDEO_LOG_ENV).is_some_and(|v| v == "1")
}

#[cfg(windows)]
pub(crate) fn send_rune_video_log() -> Result<std::path::PathBuf, std::io::Error> {
    let path = greyscreen_core::paths::logs_dir().join("rune-video.log");
    greyscreen_capture::video_log::send_rune_video_log_to(&path)?;
    Ok(path)
}

pub(crate) fn init() -> Option<WorkerGuard> {
    let dir = greyscreen_core::paths::logs_dir();
    let appender = tracing_appender::rolling::daily(dir, "greyscreen.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let default = if video_log_requested() {
        format!("info,{PLAYBACK_TARGET}=debug")
    } else {
        "info".to_owned()
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false);
    let registry = tracing_subscriber::registry().with(filter).with(file_layer);
    let initialized = if cfg!(debug_assertions) {
        registry
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .try_init()
    } else {
        registry.try_init()
    };
    install_panic_hook();
    initialized.ok().map(|()| guard)
}

fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        previous(info);
    }));
}
