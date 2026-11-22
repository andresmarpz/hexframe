use crossbeam_channel::never;
use greyscreen_core::media::MediaEvent;

use super::Control;
use super::effects::DISK_TOO_SLOW;
use crate::api::NoticeLevel;

impl Control {
    pub(super) fn on_media_stopped(&mut self, generation: u64) {
        if generation != self.media_gen {
            return;
        }
        self.stopping = false;
        let rx = std::mem::replace(&mut self.media_rx, never());
        if let Some(pending) = self.folder_start.as_mut() {
            pending.events.extend(
                rx.try_iter()
                    .filter(|e| matches!(e, MediaEvent::Packet(_) | MediaEvent::Stats(_))),
            );
            pending.media_stopped = true;
            return;
        }
        match self.recording.as_mut() {
            Some(recording) => {
                let mut overflowed = false;
                for event in rx.try_iter() {
                    match event {
                        MediaEvent::Packet(packet) => overflowed |= recording.push(packet),
                        MediaEvent::Stats(stats) => recording.on_stats(&stats, 0),
                        _ => {}
                    }
                }
                recording.media_stopped = true;
                recording.close_writer();
                if overflowed {
                    self.notice(NoticeLevel::Error, DISK_TOO_SLOW);
                }
            }
            None if self.orphan_finalize => {
                self.orphan_finalize = false;
                self.feed(greyscreen_core::session::Input::Finalized);
            }
            None => {}
        }
        self.try_complete();
    }
}
