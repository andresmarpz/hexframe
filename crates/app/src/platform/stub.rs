use std::sync::Arc;

use greyscreen_core::media::MediaSource;
use rune::AppHandle;

use crate::ui::Ui;

pub(crate) const TRASH_NAME: &str = "Trash";
pub(crate) const CAN_START_WITH_WINDOWS: bool = false;

pub(crate) struct Launch;

pub(crate) fn begin(_headless: bool) -> Option<Launch> {
    Some(Launch)
}

impl Launch {
    pub(crate) fn configure_process(&self) {}

    pub(crate) fn install_failed(&self) -> bool {
        false
    }

    pub(crate) fn serve_second_launch(&mut self, _handle: &AppHandle<Ui>) {}
}

pub(crate) fn init_dpi() {}

pub(crate) fn media_source() -> Arc<dyn MediaSource> {
    Arc::new(greyscreen_engine::FakeMediaSource::new())
}

pub(crate) fn can_install_updates() -> bool {
    false
}

pub(crate) fn connected_display_sizes() -> Vec<(u32, u32)> {
    Vec::new()
}

pub(crate) fn start_with_windows() -> bool {
    false
}

pub(crate) fn set_start_with_windows(_enabled: bool) {}
