use greyscreen_core::feedback::FeedbackResult;
use greyscreen_core::settings::{MAX_SOUND_VOLUME, Settings};
use greyscreen_engine::{EngineStatus, MANUAL_CLIP_LABEL, NoticeLevel, UiMsg};

mod output;

use output::{Output, RuneOutput, Silent};

const FAILURE_GAP_SECS: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cue {
    RecordingStarted,
    RecordingSaved,
    ClipSaved,
    Failed,
}

impl Cue {
    pub(crate) const ALL: [Cue; 4] = [
        Cue::RecordingStarted,
        Cue::RecordingSaved,
        Cue::ClipSaved,
        Cue::Failed,
    ];

    fn file_name(self) -> &'static str {
        match self {
            Cue::RecordingStarted => "recording-started.wav",
            Cue::RecordingSaved => "recording-saved.wav",
            Cue::ClipSaved => "clip-saved.wav",
            Cue::Failed => "failed.wav",
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Cue::RecordingStarted => include_bytes!("../assets/sounds/recording-started.wav"),
            Cue::RecordingSaved => include_bytes!("../assets/sounds/recording-saved.wav"),
            Cue::ClipSaved => include_bytes!("../assets/sounds/clip-saved.wav"),
            Cue::Failed => include_bytes!("../assets/sounds/failed.wav"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Sfx {
    output: Box<dyn Output>,
    recording: bool,
    last_failure: Option<f64>,
}

impl Sfx {
    pub(crate) fn silent() -> Sfx {
        Sfx::with_output(Box::new(Silent))
    }

    pub(crate) fn rune() -> Sfx {
        Sfx::with_output(Box::new(RuneOutput::start()))
    }

    fn with_output(output: Box<dyn Output>) -> Sfx {
        Sfx {
            output,
            recording: false,
            last_failure: None,
        }
    }

    pub(crate) fn on_msg(&mut self, settings: &Settings, msg: &UiMsg, now: f64) {
        let was_recording = self.recording;
        self.recording = recording_after(msg, was_recording);
        let Some(cue) = cue_for(msg, was_recording) else {
            return;
        };
        if cue == Cue::Failed && self.failure_is_repeat(now) {
            return;
        }
        self.play(settings, cue);
    }

    pub(crate) fn preview(&mut self, settings: &Settings) {
        self.play(settings, Cue::ClipSaved);
    }

    fn play(&mut self, settings: &Settings, cue: Cue) {
        if settings.sound_effects && settings.sound_volume > 0 {
            let volume =
                settings.sound_volume.min(MAX_SOUND_VOLUME) as f32 / MAX_SOUND_VOLUME as f32;
            self.output.play(cue, volume);
        }
    }

    fn failure_is_repeat(&mut self, now: f64) -> bool {
        let repeat = self
            .last_failure
            .is_some_and(|at| (0.0..FAILURE_GAP_SECS).contains(&(now - at)));
        if !repeat {
            self.last_failure = Some(now);
        }
        repeat
    }
}

fn recording_after(msg: &UiMsg, before: bool) -> bool {
    match msg {
        UiMsg::Status(status) => matches!(status, EngineStatus::Recording { .. }),
        _ => before,
    }
}

fn cue_for(msg: &UiMsg, was_recording: bool) -> Option<Cue> {
    match msg {
        UiMsg::Status(EngineStatus::Recording { .. }) if !was_recording => {
            Some(Cue::RecordingStarted)
        }
        UiMsg::Status(EngineStatus::Error(_)) => Some(Cue::Failed),
        UiMsg::RecordingSaved(_) => Some(Cue::RecordingSaved),
        UiMsg::ClipSaved(entry) if entry.clip.label == MANUAL_CLIP_LABEL => Some(Cue::ClipSaved),
        UiMsg::Notice {
            level: NoticeLevel::Error,
            ..
        }
        | UiMsg::NoticeWithAction {
            level: NoticeLevel::Error,
            ..
        }
        | UiMsg::Feedback(FeedbackResult::Failed { .. }) => Some(Cue::Failed),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod tests;
