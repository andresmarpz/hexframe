use greyscreen_core::problem::Problem;

use super::Control;
use crate::api::NoticeLevel;

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
        self.notice(NoticeLevel::Error, Problem::write_failure_notice(err));
    }
}
