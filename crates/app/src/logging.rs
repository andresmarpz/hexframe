use std::panic;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub(crate) fn init() -> Option<WorkerGuard> {
    let dir = greyscreen_core::paths::logs_dir();
    let appender = tracing_appender::rolling::daily(dir, "greyscreen.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
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
