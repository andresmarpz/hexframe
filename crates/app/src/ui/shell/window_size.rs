use greyscreen_core::settings::{MAX_WINDOW_SIZE, MIN_WINDOW_SIZE};
use rune::Tx;

use crate::ui::Ui;

const SETTLE_SECS: f64 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WindowSizeWatch {
    seen: Option<(u32, u32)>,
    settled_at: f64,
}

impl WindowSizeWatch {
    pub(crate) fn new() -> Self {
        Self {
            seen: None,
            settled_at: 0.0,
        }
    }
}

impl Ui {
    pub(crate) fn note_window_size(&mut self, width: f32, height: f32) {
        if self.window_full_screen || !width.is_finite() || !height.is_finite() {
            return;
        }
        let size = (width.round() as u32, height.round() as u32);
        let fits = size.0 >= MIN_WINDOW_SIZE.0
            && size.1 >= MIN_WINDOW_SIZE.1
            && size.0 <= MAX_WINDOW_SIZE.0
            && size.1 <= MAX_WINDOW_SIZE.1;
        if fits && self.window_watch.seen != Some(size) {
            self.window_watch.seen = Some(size);
            self.window_watch.settled_at = f64::INFINITY;
        }
    }

    pub(crate) fn save_settled_window_size(&mut self, clock: f64, tx: &mut Tx) {
        let Some(size) = self.window_watch.seen else {
            return;
        };
        if self.settings.window_size == Some(size) {
            return;
        }
        if self.window_watch.settled_at.is_infinite() {
            self.window_watch.settled_at = clock + SETTLE_SECS;
        }
        if clock >= self.window_watch.settled_at {
            self.change(|s| s.window_size = Some(size));
        } else {
            tx.wake_at(self.window_watch.settled_at);
        }
    }
}

#[cfg(test)]
mod tests;
