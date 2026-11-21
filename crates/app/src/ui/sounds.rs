use greyscreen_core::settings::MAX_SOUND_VOLUME;
use greyscreen_engine::EngineCmd;

use super::Ui;

const VOLUME_STEP: i32 = 5;

impl Ui {
    pub(crate) fn drag_sound_volume(&mut self, percent: u32) {
        self.settings.sound_volume = percent.min(MAX_SOUND_VOLUME);
    }

    pub(crate) fn finish_sound_volume(&mut self) {
        self.send(EngineCmd::ApplySettings(self.settings.clone()));
        self.sfx.preview(&self.settings);
    }

    pub(crate) fn step_sound_volume(&mut self, steps: i32) {
        let next = (self.settings.sound_volume as i32 / VOLUME_STEP + steps) * VOLUME_STEP;
        self.drag_sound_volume(next.clamp(0, MAX_SOUND_VOLUME as i32) as u32);
        self.finish_sound_volume();
    }
}
