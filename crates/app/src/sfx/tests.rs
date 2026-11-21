use std::sync::{Arc, Mutex};

use greyscreen_core::feedback::FeedbackResult;
use greyscreen_core::library::ClipEntry as ClipMeta;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{ClipEntry, EngineStatus, MANUAL_CLIP_LABEL, NoticeLevel, UiMsg};

use super::output::{Output, RuneOutput, install};
use super::{Cue, Sfx};

pub(crate) type Played = Arc<Mutex<Vec<(Cue, f32)>>>;

#[derive(Debug)]
struct Capture(Played);

impl Output for Capture {
    fn play(&mut self, cue: Cue, volume: f32) {
        self.0.lock().unwrap().push((cue, volume));
    }
}

pub(crate) fn capture() -> (Sfx, Played) {
    let played = Played::default();
    (Sfx::with_output(Box::new(Capture(played.clone()))), played)
}

fn cues(played: &Played) -> Vec<Cue> {
    played.lock().unwrap().iter().map(|(cue, _)| *cue).collect()
}

fn recording() -> UiMsg {
    UiMsg::Status(EngineStatus::Recording {
        since: 0,
        game: None,
    })
}

fn clip(label: &str) -> UiMsg {
    UiMsg::ClipSaved(ClipEntry {
        clip: ClipMeta {
            file: "clips/a.mp4".into(),
            start_secs: 1.0,
            end_secs: 5.0,
            label: label.into(),
        },
        folder: "game".into(),
    })
}

fn notice(level: NoticeLevel) -> UiMsg {
    UiMsg::Notice {
        level,
        text: "x".into(),
    }
}

fn feed(sfx: &mut Sfx, messages: &[UiMsg]) {
    let settings = Settings::default();
    for (i, msg) in messages.iter().enumerate() {
        sfx.on_msg(&settings, msg, i as f64 * 10.0);
    }
}

#[test]
fn each_moment_plays_its_own_sound() {
    let (mut sfx, played) = capture();
    feed(
        &mut sfx,
        &[
            recording(),
            clip(MANUAL_CLIP_LABEL),
            UiMsg::RecordingSaved(crate::ui::review_entry()),
            notice(NoticeLevel::Error),
            UiMsg::Feedback(FeedbackResult::Failed {
                error: "x".into(),
                report: "{}".into(),
            }),
            UiMsg::Status(EngineStatus::Error("x".into())),
        ],
    );
    assert_eq!(
        cues(&played),
        [
            Cue::RecordingStarted,
            Cue::ClipSaved,
            Cue::RecordingSaved,
            Cue::Failed,
            Cue::Failed,
            Cue::Failed,
        ]
    );
}

#[test]
fn everything_else_is_silent() {
    let (mut sfx, played) = capture();
    feed(
        &mut sfx,
        &[
            clip("Kill"),
            notice(NoticeLevel::Info),
            notice(NoticeLevel::Warn),
            UiMsg::Status(EngineStatus::Idle),
            UiMsg::Status(EngineStatus::Waiting),
            UiMsg::Status(EngineStatus::Finalizing),
            UiMsg::Feedback(FeedbackResult::Sent),
            UiMsg::Library(Vec::new()),
        ],
    );
    assert!(cues(&played).is_empty());
}

#[test]
fn recording_starts_once_per_recording() {
    let (mut sfx, played) = capture();
    feed(
        &mut sfx,
        &[
            recording(),
            recording(),
            UiMsg::Status(EngineStatus::Finalizing),
            recording(),
        ],
    );
    assert_eq!(
        cues(&played),
        [Cue::RecordingStarted, Cue::RecordingStarted]
    );
}

#[test]
fn nothing_plays_with_sound_effects_off() {
    let (mut sfx, played) = capture();
    let settings = Settings {
        sound_effects: false,
        ..Settings::default()
    };
    for msg in [
        recording(),
        clip(MANUAL_CLIP_LABEL),
        notice(NoticeLevel::Error),
    ] {
        sfx.on_msg(&settings, &msg, 0.0);
    }
    sfx.preview(&settings);
    assert!(cues(&played).is_empty());
}

#[test]
fn a_recording_that_began_while_muted_does_not_ring_again_when_unmuted() {
    let (mut sfx, played) = capture();
    let muted = Settings {
        sound_effects: false,
        ..Settings::default()
    };
    sfx.on_msg(&muted, &recording(), 0.0);
    sfx.on_msg(&Settings::default(), &recording(), 1.0);
    assert!(cues(&played).is_empty());
}

#[test]
fn zero_volume_is_silent_and_the_volume_scales_the_sound() {
    let (mut sfx, played) = capture();
    let mut settings = Settings {
        sound_volume: 0,
        ..Settings::default()
    };
    sfx.on_msg(&settings, &clip(MANUAL_CLIP_LABEL), 0.0);
    assert!(played.lock().unwrap().is_empty());
    settings.sound_volume = 25;
    sfx.on_msg(&settings, &clip(MANUAL_CLIP_LABEL), 0.0);
    assert_eq!(*played.lock().unwrap(), [(Cue::ClipSaved, 0.25)]);
}

#[test]
fn a_burst_of_failures_makes_one_sound() {
    let (mut sfx, played) = capture();
    let settings = Settings::default();
    for at in [0.0, 0.2, 1.5, 2.5] {
        sfx.on_msg(&settings, &notice(NoticeLevel::Error), at);
    }
    assert_eq!(cues(&played), [Cue::Failed, Cue::Failed]);
}

#[test]
fn every_cue_has_a_wav_file() {
    for cue in Cue::ALL {
        let bytes = cue.bytes();
        assert_eq!(&bytes[..4], b"RIFF", "{cue:?}");
        assert!(bytes.len() < 100_000, "{cue:?}");
    }
}

#[test]
fn installing_writes_the_files_and_repairs_a_stale_one() {
    let dir = tempfile::tempdir().unwrap();
    install(dir.path()).unwrap();
    let stale = dir.path().join(Cue::Failed.file_name());
    std::fs::write(&stale, b"old").unwrap();
    install(dir.path()).unwrap();
    for cue in Cue::ALL {
        assert_eq!(
            std::fs::read(dir.path().join(cue.file_name())).unwrap(),
            cue.bytes()
        );
    }
}

#[test]
fn an_unwritable_folder_only_logs() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"").unwrap();
    let mut output = RuneOutput::in_dir(blocker.join("sounds"));
    output.play(Cue::ClipSaved, 1.0);
    output.play(Cue::Failed, 1.0);
    assert_eq!(output.playing(), 0);
}

#[test]
fn a_missing_file_only_logs() {
    let dir = tempfile::tempdir().unwrap();
    let mut output = RuneOutput::in_dir(dir.path().to_path_buf());
    output.play(Cue::Failed, 1.0);
    std::fs::remove_file(dir.path().join(Cue::ClipSaved.file_name())).unwrap();
    output.play(Cue::ClipSaved, 1.0);
    assert_eq!(output.playing(), 1);
}

#[test]
fn a_broken_file_is_reported_and_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let mut output = RuneOutput::in_dir(dir.path().to_path_buf());
    output.play(Cue::Failed, 1.0);
    output.reap();
    std::fs::write(dir.path().join(Cue::ClipSaved.file_name()), b"not audio").unwrap();
    output.play(Cue::ClipSaved, 1.0);
    assert_eq!(output.playing(), 2);
    assert_eq!(output.reap().len(), 1);
    assert_eq!(output.playing(), 1);
}

#[test]
fn good_files_overlap_without_failing() {
    let dir = tempfile::tempdir().unwrap();
    let mut output = RuneOutput::in_dir(dir.path().to_path_buf());
    output.play(Cue::ClipSaved, 0.5);
    output.play(Cue::ClipSaved, 0.5);
    output.play(Cue::Failed, 0.5);
    assert_eq!(output.playing(), 3);
    assert!(output.reap().is_empty());
}
