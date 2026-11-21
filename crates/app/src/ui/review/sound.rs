use super::ReviewState;

pub(crate) const SPEEDS: [f32; 7] = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
pub(crate) const NORMAL_SPEED: usize = 3;
const UNMUTE_VOLUME: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Sound {
    pub volume: f32,
    pub muted: bool,
}

impl Default for Sound {
    fn default() -> Self {
        Sound {
            volume: 1.0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct VolumeHover {
    pub button: bool,
    pub popover: bool,
    pub dragging: bool,
}

impl ReviewState {
    pub(crate) fn sound(&self) -> Sound {
        Sound {
            volume: self.volume,
            muted: self.muted,
        }
    }

    pub(crate) fn set_speed(&mut self, index: usize) {
        self.speed = index.min(SPEEDS.len() - 1);
        self.speed_menu = false;
        if let Some(player) = &self.player {
            player.set_rate(self.speed_rate());
        }
    }

    pub(crate) fn step_speed(&mut self, delta: i32) {
        let last = SPEEDS.len() - 1;
        let next = self.speed.saturating_add_signed(delta as isize).min(last);
        if next != self.speed {
            self.set_speed(next);
        }
    }

    pub(crate) fn toggle_mute(&mut self) {
        let silent = self.muted || self.volume <= 0.0;
        self.muted = !silent;
        if silent && self.volume <= 0.0 {
            self.volume = UNMUTE_VOLUME;
            self.push_volume();
        }
        self.push_muted();
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        let was_muted = self.muted;
        self.volume = volume.clamp(0.0, 1.0);
        self.muted = self.volume <= 0.0;
        self.push_volume();
        if self.muted != was_muted {
            self.push_muted();
        }
    }

    pub(crate) fn shown_volume(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume }
    }

    pub(crate) fn volume_open(&self) -> bool {
        let hover = self.volume_hover;
        hover.button || hover.popover || hover.dragging
    }

    pub(crate) fn controls_menu_open(&self) -> bool {
        self.speed_menu || self.volume_open()
    }

    fn push_volume(&self) {
        if let Some(player) = &self.player {
            player.set_volume(self.volume);
        }
    }

    fn push_muted(&self) {
        if let Some(player) = &self.player {
            player.set_muted(self.muted);
        }
    }
}
