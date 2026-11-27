use std::cell::RefCell;

use greyscreen_core::settings::{MAX_WINDOW_SIZE, MIN_WINDOW_SIZE};
use rune::{Tx, WindowHandle, WindowPlacement};

use crate::ui::Ui;

const SETTLE_SECS: f64 = 0.8;

thread_local! {
    static WINDOW: RefCell<Option<WindowHandle>> = const { RefCell::new(None) };
}

pub(crate) fn remember_window(handle: &WindowHandle) {
    WINDOW.with_borrow_mut(|slot| *slot = Some(handle.clone()));
}

fn placement() -> Option<WindowPlacement> {
    WINDOW.with_borrow(|slot| slot.as_ref().and_then(WindowHandle::placement))
}

fn fits(size: (u32, u32)) -> bool {
    size.0 >= MIN_WINDOW_SIZE.0
        && size.1 >= MIN_WINDOW_SIZE.1
        && size.0 <= MAX_WINDOW_SIZE.0
        && size.1 <= MAX_WINDOW_SIZE.1
}

fn points((width, height): (u32, u32), scale: f64) -> (u32, u32) {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    (
        (f64::from(width) / scale).round() as u32,
        (f64::from(height) / scale).round() as u32,
    )
}

fn resolve(
    placement: Option<WindowPlacement>,
    seen: (u32, u32),
    saved: Option<(u32, u32)>,
) -> Option<((u32, u32), bool)> {
    let Some(p) = placement else {
        return Some((seen, false));
    };
    if p.full_screen || p.minimized {
        return None;
    }
    if p.maximized {
        let restored = p.restore.map(|(_, size)| points(size, p.scale));
        return Some((restored.or(saved).unwrap_or(seen), true));
    }
    Some((points(p.content_size, p.scale), false))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WindowSizeWatch {
    seen: Option<(u32, u32)>,
    settled_at: f64,
}

impl WindowSizeWatch {
    pub(crate) fn new() -> Self {
        Self {
            seen: None,
            settled_at: f64::INFINITY,
        }
    }
}

impl Ui {
    pub(crate) fn note_window_size(&mut self, width: f32, height: f32) {
        if self.window_full_screen || !width.is_finite() || !height.is_finite() {
            return;
        }
        let size = (width.round() as u32, height.round() as u32);
        if fits(size) && self.window_watch.seen != Some(size) {
            self.window_watch.seen = Some(size);
            self.window_watch.settled_at = f64::INFINITY;
        }
    }

    pub(crate) fn save_settled_window_size(&mut self, clock: f64, tx: &mut Tx) {
        let Some(seen) = self.window_watch.seen else {
            return;
        };
        if self.window_watch.settled_at.is_infinite() {
            self.window_watch.settled_at = clock + SETTLE_SECS;
        }
        if clock < self.window_watch.settled_at {
            tx.wake_at(self.window_watch.settled_at);
            return;
        }
        self.window_watch.seen = None;
        let Some((size, maximized)) = resolve(placement(), seen, self.settings.window_size) else {
            return;
        };
        if Some(size) != self.settings.window_size || maximized != self.settings.window_maximized {
            self.change(|s| {
                s.window_size = Some(size);
                s.window_maximized = maximized;
            });
        }
    }
}
