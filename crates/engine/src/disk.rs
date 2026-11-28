use std::path::Path;
use std::time::Duration;

use greyscreen_core::disk::{CHECK_EVERY_SECS, gb_text};

use crate::api::{NoticeAction, NoticeLevel, UiMsg};

pub const LOW_DISK_URL: &str = "https://greyscreen.app/faq#low-disk";
pub(crate) const REFUSED_REASON: &str = "Not enough disk space to record";

pub trait FreeSpace: Send + Sync {
    fn free_bytes(&self, dir: &Path) -> Option<u64>;

    fn check_every(&self) -> Duration {
        Duration::from_secs(CHECK_EVERY_SECS)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SystemFreeSpace;

impl FreeSpace for SystemFreeSpace {
    fn free_bytes(&self, dir: &Path) -> Option<u64> {
        let existing = dir.ancestors().find(|path| path.exists())?;
        match fs4::available_space(existing) {
            Ok(bytes) => Some(bytes),
            Err(err) => {
                tracing::warn!(%err, dir = %existing.display(), "cannot read the free disk space");
                None
            }
        }
    }
}

fn learn_more(text: String) -> UiMsg {
    UiMsg::NoticeWithAction {
        level: NoticeLevel::Error,
        text,
        action: NoticeAction::Link(LOW_DISK_URL.to_owned()),
    }
}

pub(crate) fn refused(free_bytes: u64) -> UiMsg {
    learn_more(format!(
        "Not enough disk space to record ({} free). Free up space or change the recordings folder.",
        gb_text(free_bytes)
    ))
}

pub(crate) fn stopped(free_bytes: u64) -> UiMsg {
    learn_more(format!(
        "Recording stopped: the disk is almost full ({} free). Free up space or change the recordings folder.",
        gb_text(free_bytes)
    ))
}
