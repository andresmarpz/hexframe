use rune::Tx;

use super::{Screen, Ui};

const AWAKE_SECS: f64 = 2.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DockState {
    pub visible: bool,
    hovered: bool,
    awake_since: Option<f64>,
}

impl Default for DockState {
    fn default() -> Self {
        DockState {
            visible: true,
            hovered: false,
            awake_since: None,
        }
    }
}

impl Ui {
    pub(crate) fn dock_hover(&mut self, inside: bool, tx: &mut Tx) {
        self.dock.hovered = inside;
        self.wake_dock(tx.clock());
    }

    pub(crate) fn wake_dock(&mut self, clock: f64) {
        self.dock.awake_since = Some(clock);
        self.sync_dock(clock);
    }

    pub(crate) fn sync_dock(&mut self, clock: f64) -> bool {
        let playing =
            self.screen == Screen::Games && self.review.as_ref().is_some_and(|r| r.is_playing());
        if !playing {
            self.dock.awake_since = None;
        } else if self.dock.awake_since.is_none() {
            self.dock.awake_since = Some(clock);
        }
        let recent = self
            .dock
            .awake_since
            .is_none_or(|since| clock - since < AWAKE_SECS);
        let visible = !playing || self.dock.hovered || recent;
        let changed = visible != self.dock.visible;
        self.dock.visible = visible;
        changed
    }
}
