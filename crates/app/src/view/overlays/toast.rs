use greyscreen_engine::NoticeLevel;
use rune::El;
use rune_kit::{Kit, Toast, ToastRegion};

use crate::ui::{Notice, Ui};

pub(crate) fn toast(notice: &Notice, k: &Kit) -> El {
    let built = match notice.level {
        NoticeLevel::Info => Toast::new("notice", notice.text.clone()),
        NoticeLevel::Warn => Toast::new("notice", "Warning").description(notice.text.clone()),
        NoticeLevel::Error => Toast::new("notice", "Error").description(notice.text.clone()),
    };
    let built = built.closable().on_close(|ui: &mut Ui| ui.notice = None);
    ToastRegion::new().toast(built.build(k)).build(k)
}
