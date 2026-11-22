use std::path::Path;

use greyscreen_engine::{EngineHandle, abandon_encoder_check};

use crate::tray::TrayOwner;

pub(crate) fn finish(
    tray: Option<TrayOwner>,
    engine: Option<EngineHandle>,
    encoder_check_cache: &Path,
) {
    if let Some(tray) = tray {
        tray.close();
    }
    drop(engine);
    abandon_encoder_check(encoder_check_cache);
}
