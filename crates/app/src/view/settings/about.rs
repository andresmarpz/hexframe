use greyscreen_core::paths;
use greyscreen_engine::reveal;
use rune::{El, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::catalog::Row;
use super::{setting_row, toggle_row, updates};
use crate::ui::Ui;

const RIOT_NOTICE: &str = "Greyscreen isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.";

pub(super) fn render(s: &Ui, k: &Kit, which: Row) -> El {
    match which {
        Row::Version => version(k),
        Row::Updates => updates::update_row(s, k),
        Row::AutoUpdates => updates::auto_row(s, k),
        Row::Logs => logs(k),
        Row::Feedback => feedback(k),
        Row::StartWithWindows => toggle_row(
            k,
            "start-with-windows",
            "Start with Windows",
            Some("Open in the tray when you sign in."),
            s.start_with_windows,
            Ui::toggle_start_with_windows,
        ),
        _ => notice(k),
    }
}

fn version(k: &Kit) -> El {
    let version = text(format!("Version {}", env!("CARGO_PKG_VERSION")))
        .key("version")
        .font(k.ty.body)
        .color(k.c.text_muted);
    setting_row(k, "version-row", "Greyscreen", None, version)
}

fn logs(k: &Kit) -> El {
    let button = Button::new("Open logs folder")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|_: &mut Ui| reveal(&paths::logs_dir()))
        .build(k)
        .key("open-logs");
    setting_row(
        k,
        "logs-row",
        "Logs",
        Some("Attach these when you report a problem."),
        button,
    )
}

fn feedback(k: &Kit) -> El {
    let button = Button::new("Send feedback")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.open_feedback(tx))
        .build(k)
        .key("send-feedback");
    setting_row(
        k,
        "feedback-row",
        "Feedback",
        Some("Report a bug or share an idea."),
        button,
    )
}

fn notice(k: &Kit) -> El {
    row().key("riot-notice").w_full().px(16.0).py(10.0).child(
        text(RIOT_NOTICE)
            .wrap()
            .w_full()
            .font(k.ty.small)
            .color(k.c.text_muted),
    )
}
