use greyscreen_core::problem::Problem;

use super::Control;
use crate::api::{NoticeAction, NoticeLevel, UiMsg};

impl Control {
    pub(super) fn on_writer_failed(&mut self, err: &str) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        if recording.write_failed {
            return;
        }
        recording.write_failed = true;
        recording.health.on_write_failure();
        recording.close_writer();
        self.send(UiMsg::NoticeWithAction {
            level: NoticeLevel::Error,
            text: Problem::write_failure_notice(err).to_owned(),
            action: NoticeAction::Link(Problem::from_error(err).help_url()),
        });
    }
}
