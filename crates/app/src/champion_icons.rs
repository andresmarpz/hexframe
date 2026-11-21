use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use greyscreen_core::paths;
use greyscreen_league::ddragon::{self, DEFAULT_BASE};
use rune::{AppHandle, Tx};

use crate::ui::Ui;
use crate::view::champion;

pub(crate) fn start_prefetch(handle: AppHandle<Ui>) {
    let spawned = thread::Builder::new()
        .name("champion-icons".into())
        .spawn(move || prefetch(&handle));
    if let Err(err) = spawned {
        tracing::warn!(%err, "cannot start champion icon download");
    }
}

fn prefetch(handle: &AppHandle<Ui>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    match ddragon::sync(DEFAULT_BASE, &paths::champion_icons_dir(), now) {
        Ok(report) => {
            tracing::info!(
                version = %report.version,
                downloaded = report.downloaded,
                failed = report.failed,
                "champion icons ready"
            );
            if report.downloaded > 0 || report.checked_versions {
                handle.update(|_: &mut Ui, _: &mut Tx| champion::reload());
            }
        }
        Err(err) => tracing::warn!(%err, "cannot update champion icons"),
    }
}
