use greyscreen_engine::{NoticeAction, NoticeLevel};
use rune::{El, Tx};
use rune_kit::{Kit, Toast, ToastRegion};

use crate::headless::reveal;
use crate::ui::{Notice, Ui};

pub(crate) fn toast(notice: &Notice, k: &Kit) -> El {
    let built = match notice.level {
        NoticeLevel::Info => Toast::new("notice", notice.text.clone()),
        NoticeLevel::Warn => Toast::new("notice", "Warning").description(notice.text.clone()),
        NoticeLevel::Error => Toast::new("notice", "Error").description(notice.text.clone()),
    };
    let built = match notice.action.clone() {
        Some(NoticeAction::Reveal(folder)) => {
            built.action(k, "Show in folder", false, move |_: &mut Ui, _: &mut Tx| {
                reveal(&folder);
            })
        }
        Some(NoticeAction::Link(url)) => {
            built.action(k, "Learn more", false, move |_: &mut Ui, tx: &mut Tx| {
                if let Err(err) = tx.open_url(&url) {
                    tracing::warn!(%err, "cannot open the help page");
                }
            })
        }
        None => built,
    };
    let built = built.closable().on_close(|ui: &mut Ui| ui.notice = None);
    ToastRegion::new().toast(built.build(k)).build(k)
}
