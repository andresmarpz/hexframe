use std::time::Duration;

use greyscreen_engine::{EngineCmd, EngineStatus, NoticeAction, NoticeLevel, UiMsg};
use rune::Tx;

use super::{FEED_CAP, Notice, STOP_CONFIRM_AFTER_SECS, Ui};
use crate::tray::TrayState;

const STOP_CONFIRM_SECS: f64 = 3.0;
const NOTICE_SECS: f64 = 4.0;
const NOTICE_ACTION_SECS: f64 = 12.0;
const TICK: Duration = Duration::from_millis(250);
const PLAYBACK_TICK: Duration = Duration::from_millis(33);

impl Ui {
    pub(crate) fn request_stop(&mut self, tx: &mut Tx) {
        let now = tx.clock();
        let confirmed = self.stop_confirm_until.is_some_and(|until| now < until);
        if self.elapsed_secs <= STOP_CONFIRM_AFTER_SECS || confirmed {
            self.stop_confirm_until = None;
            self.send(EngineCmd::StopRecording);
        } else {
            let until = now + STOP_CONFIRM_SECS;
            self.stop_confirm_until = Some(until);
            tx.wake_at(until);
        }
    }

    pub(crate) fn sync_tray(&self) {
        if let Some(tray) = &self.tray {
            tray.send(TrayState {
                status: self.status.clone(),
                auto_record: self.settings.auto_record,
            });
        }
    }

    pub(crate) fn retry_capture(&self) {
        self.send(EngineCmd::Retry);
    }

    pub(crate) fn dismiss_error(&mut self) {
        if matches!(self.status, EngineStatus::Error(_)) {
            self.status = EngineStatus::Idle;
        }
    }

    pub(crate) fn apply(&mut self, msg: UiMsg, tx: &mut Tx) {
        self.sfx.on_msg(&self.settings, &msg, tx.clock());
        match msg {
            UiMsg::Status(status) => self.apply_status(status, tx),
            UiMsg::Feed(item) => {
                self.feed.push_back(item);
                while self.feed.len() > FEED_CAP {
                    self.feed.pop_front();
                }
            }
            UiMsg::ClipSaved(entry) => {
                let text = format!("Clip saved: {}", entry.clip.label);
                self.show_notice(NoticeLevel::Info, text, tx);
            }
            UiMsg::RecordingSaved(entry) => {
                self.first_test_saved(&entry);
                let text = format!("Recording saved: {}", entry.meta.game.info.me.champion);
                self.last_game = Some(entry.clone());
                match self
                    .recordings
                    .iter_mut()
                    .find(|r| r.meta.id == entry.meta.id)
                {
                    Some(slot) => *slot = entry,
                    None => self.recordings.insert(0, entry),
                }
                self.show_notice(NoticeLevel::Info, text, tx);
            }
            UiMsg::Library(recordings) => self.apply_library(recordings),
            UiMsg::Thumbnail(path) => crate::view::thumb::invalidate(&path),
            UiMsg::Notice { level, text } => self.show_notice(level, text, tx),
            UiMsg::NoticeWithAction {
                level,
                text,
                action,
            } => self.show_notice_with(level, text, Some(action), tx),
            UiMsg::Update(state) => self.apply_update(state),
            UiMsg::Feedback(result) => self.apply_feedback(result),
            UiMsg::Export(event) => self.apply_export(event),
            UiMsg::HotkeyStatus { errors } => self.hotkey_errors = errors,
            UiMsg::Encoder(check) => self.encoder = Some(check),
        }
    }

    fn apply_status(&mut self, status: EngineStatus, tx: &mut Tx) {
        let was_recording = matches!(self.status, EngineStatus::Recording { .. });
        match &status {
            EngineStatus::Recording { .. } if !was_recording => {
                self.feed.clear();
                self.last_game = None;
                self.recording_since_clock = Some(tx.clock());
                self.elapsed_secs = 0;
                self.live_selected = self.review.is_none();
                tx.wake_after(TICK);
            }
            EngineStatus::Recording { .. } => {}
            EngineStatus::Idle | EngineStatus::GameRunning => {
                self.recording_since_clock = None;
                self.feed.clear();
            }
            _ => self.recording_since_clock = None,
        }
        if !matches!(status, EngineStatus::Recording { .. }) {
            self.stop_confirm_until = None;
        }
        self.first_test_status(&status);
        self.status = status;
        if !self.live_row_visible() {
            self.live_selected = false;
        }
        self.install_if_idle();
        self.sync_tray();
    }

    pub(crate) fn show_notice(&mut self, level: NoticeLevel, text: String, tx: &mut Tx) {
        self.show_notice_with(level, text, None, tx);
    }

    pub(crate) fn show_notice_with(
        &mut self,
        level: NoticeLevel,
        text: String,
        action: Option<NoticeAction>,
        tx: &mut Tx,
    ) {
        let secs = if action.is_some() {
            NOTICE_ACTION_SECS
        } else {
            NOTICE_SECS
        };
        let until = tx.clock() + secs;
        self.notice = Some((
            Notice {
                level,
                text,
                action,
            },
            until,
        ));
        tx.wake_at(until);
    }

    pub(crate) fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        self.sync_full_screen(tx);
        if let Some((_, until)) = &self.notice
            && clock >= *until
        {
            self.notice = None;
            changed = true;
        }
        if let Some(until) = self.stop_confirm_until
            && clock >= until
        {
            self.stop_confirm_until = None;
            changed = true;
        }
        if let Some(start) = self.recording_since_clock {
            let secs = (clock - start).max(0.0) as u32;
            if secs != self.elapsed_secs {
                self.elapsed_secs = secs;
                changed = true;
            }
            tx.wake_after(TICK);
        }
        changed |= self.sync_dock(clock);
        if let Some(review) = &mut self.review {
            review.tick(clock);
            changed |= review.death_tick();
            changed |= review.changed_since_shown();
            changed |= review.hide_idle_controls(clock);
            if review.needs_ticks() {
                tx.wake_after(PLAYBACK_TICK);
            }
        }
        changed
    }
}
