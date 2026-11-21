use greyscreen_core::update::UpdateInfo;
use rune::{El, row};
use rune_kit::{Kit, Toast};

use crate::ui::Ui;

const TOP_GAP: f32 = 52.0;

pub(super) fn layer(s: &Ui, k: &Kit) -> Option<El> {
    let info = s.update_banner()?;
    Some(
        row()
            .key("update-banner-layer")
            .pin(Some(0.0), Some(TOP_GAP), Some(0.0), None)
            .justify_center()
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
