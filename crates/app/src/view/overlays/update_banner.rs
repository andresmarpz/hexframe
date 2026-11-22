use greyscreen_core::settings::NavStyle;
use greyscreen_core::update::UpdateInfo;
use rune::{El, row};
use rune_kit::{Kit, Toast};

use crate::ui::Ui;

const EDGE_GAP: f32 = 16.0;
const DOCK_GAP: f32 = 76.0;
const NOTICE_GAP: f32 = 80.0;

pub(crate) fn layer(s: &Ui, k: &Kit) -> Option<El> {
    let info = s.update_banner()?;
    let bottom_gap = if s.notice.is_some() {
        NOTICE_GAP
    } else if s.settings.nav_style == NavStyle::Dock {
        DOCK_GAP
    } else {
        EDGE_GAP
    };
    Some(
        row()
            .key("update-banner-layer")
            .pin(None, None, Some(EDGE_GAP), Some(bottom_gap))
            .child(banner(info, k)),
    )
}

fn banner(info: &UpdateInfo, k: &Kit) -> El {
    Toast::new(
        "update-banner",
        format!("Greyscreen {} is ready", info.version()),
    )
    .action(k, "Restart to update", true, |ui: &mut Ui, _| {
        ui.install_update()
    })
    .action(k, "Later", false, |ui: &mut Ui, _| {
        ui.dismiss_update_banner()
    })
    .build(k)
}
