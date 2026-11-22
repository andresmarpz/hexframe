use crossbeam_channel::unbounded;
use greyscreen_core::library::ClipEntry as ClipMeta;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{ClipEntry, EngineCmd, EngineStatus, MANUAL_CLIP_LABEL, NoticeLevel, UiMsg};
use rune::Runtime;

use super::{rect, runtime_with, settle, tap};
use crate::sfx::Cue;
use crate::sfx::tests::{Played, capture};
use crate::ui::{SettingsSection, Ui};

fn rig(settings: Settings) -> (Runtime<Ui>, Played, crossbeam_channel::Receiver<EngineCmd>) {
    let (cmd, rx) = unbounded();
    let mut ui = Ui::live(settings, cmd);
    let (sfx, played) = capture();
    ui.sfx = sfx;
    (runtime_with(ui), played, rx)
}

fn deliver(rt: &mut Runtime<Ui>, msg: UiMsg) {
    rt.update(move |ui, tx| ui.apply(msg, tx));
    rt.frame(1.0 / 60.0);
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

fn open_general_settings(rt: &mut Runtime<Ui>) {
    rt.update(|ui, _| ui.open_settings(SettingsSection::General));
    settle(rt);
}

#[test]
fn engine_messages_play_sounds_with_the_window_in_any_state() {
    let (mut rt, played, _rx) = rig(Settings::default());
    deliver(&mut rt, recording());
    deliver(&mut rt, UiMsg::RecordingSaved(crate::ui::review_entry()));
    deliver(
        &mut rt,
        UiMsg::Notice {
            level: NoticeLevel::Error,
            text: "Delete failed".into(),
        },
    );
    assert_eq!(
        cues(&played),
        [Cue::RecordingStarted, Cue::RecordingSaved, Cue::Failed]
    );
}

#[test]
fn only_a_manual_clip_plays_the_clip_sound() {
    let (mut rt, played, _rx) = rig(Settings::default());
    let clip = |label: &str| {
        UiMsg::ClipSaved(ClipEntry {
            clip: ClipMeta {
                file: "clips/a.mp4".into(),
                start_secs: 1.0,
                end_secs: 5.0,
                label: label.into(),
            },
            folder: "game".into(),
        })
    };
    deliver(&mut rt, clip("Kill"));
    deliver(&mut rt, clip(MANUAL_CLIP_LABEL));
    assert_eq!(cues(&played), [Cue::ClipSaved]);
}

#[test]
fn turning_the_switch_off_silences_everything() {
    let (mut rt, played, rx) = rig(Settings::default());
    open_general_settings(&mut rt);
    tap(&mut rt, "sound-effects-switch");
    assert!(!rt.state.settings.sound_effects);
    assert!(matches!(
        rx.try_iter().last(),
        Some(EngineCmd::ApplySettings(s)) if !s.sound_effects
    ));
    deliver(&mut rt, recording());
    assert!(cues(&played).is_empty());
}

#[test]
fn the_volume_slider_sets_the_volume_and_previews_it_on_release() {
    let (mut rt, played, rx) = rig(Settings::default());
    open_general_settings(&mut rt);
    rt.update(|_, tx| tx.scroll_into_view("sound-volume-slider".to_owned()));
    settle(&mut rt);
    let [x, y, w, h] = rect(&rt, "sound-volume-slider");
    rt.pointer_down(x + w * 0.5, y + h / 2.0);
    assert!(cues(&played).is_empty());
    rt.pointer_up(x + w * 0.5, y + h / 2.0);
    rt.frame(1.0 / 60.0);
    let volume = rt.state.settings.sound_volume;
    assert!((40..=60).contains(&volume), "volume {volume}");
    assert_eq!(cues(&played), [Cue::ClipSaved]);
    let saved: Vec<_> = rx.try_iter().collect();
    assert!(matches!(
        saved.last(),
        Some(EngineCmd::ApplySettings(s)) if s.sound_volume == volume
    ));
}

#[test]
fn the_slider_steps_and_ends_stay_in_range() {
    let (mut rt, _played, _rx) = rig(Settings::default());
    rt.update(|ui, _| ui.step_sound_volume(1));
    assert_eq!(rt.state.settings.sound_volume, 65);
    rt.update(|ui, _| ui.step_sound_volume(-100));
    assert_eq!(rt.state.settings.sound_volume, 0);
    rt.update(|ui, _| ui.step_sound_volume(100));
    assert_eq!(rt.state.settings.sound_volume, 100);
}

#[test]
fn sound_rows_are_searchable() {
    let (mut rt, _played, _rx) = rig(Settings::default());
    rt.update(|ui, _| {
        ui.open_settings(SettingsSection::Storage);
        ui.settings_search = rune::TextEdit::new("volume");
    });
    settle(&mut rt);
    assert!(rt.find("sound-volume-row").is_some());
    rt.update(|ui, _| ui.settings_search = rune::TextEdit::new("sounds"));
    settle(&mut rt);
    assert!(rt.find("sound-effects").is_some());
}
