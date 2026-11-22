use greyscreen_core::problem::NO_FRAMES;
use greyscreen_core::settings::Quality;
use greyscreen_engine::{EngineCmd, EngineStatus, UiMsg};

use super::{live_runtime, settle, tap, visible_texts};
use crate::ui::{TestRun, Ui};

fn first_run_runtime() -> (rune::Runtime<Ui>, crossbeam_channel::Receiver<EngineCmd>) {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| {
        ui.recordings.clear();
        ui.settings.first_run_done = false;
    });
    settle(&mut rt);
    (rt, rx)
}

fn has_text(rt: &rune::Runtime<Ui>, needle: &str) -> bool {
    visible_texts(rt).iter().any(|t| t.contains(needle))
}

#[test]
fn the_card_shows_on_the_first_launch_and_names_its_three_steps() {
    let (rt, _rx) = first_run_runtime();
    assert!(rt.find("first-run").is_some());
    assert!(has_text(&rt, "Where recordings are saved"));
    assert!(rt.find("first-run-folder").is_some());
    assert!(rt.find("first-run-quality").is_some());
    assert!(rt.find("first-run-test").is_some());
    assert!(rt.find("test-recording").is_none());
    assert!(rt.find("this-week").is_none());
}

#[test]
fn the_card_stays_away_once_the_first_run_is_done() {
    let (mut rt, _rx) = live_runtime();
    settle(&mut rt);
    assert!(rt.find("first-run").is_none());
    assert!(rt.find("test-recording").is_some());
}

#[test]
fn done_hides_the_card_for_good_and_saves_the_setting() {
    let (mut rt, rx) = first_run_runtime();
    tap(&mut rt, "first-run-done");
    settle(&mut rt);
    assert!(rt.find("first-run").is_none());
    let Ok(EngineCmd::ApplySettings(sent)) = rx.try_recv() else {
        panic!("settings were not sent");
    };
    assert!(sent.first_run_done);
}

#[test]
fn the_quality_control_changes_the_setting() {
    let (mut rt, rx) = first_run_runtime();
    let (x, y) = rt.find("first-run-quality").expect("quality control");
    rt.pointer_down(x + 60.0, y);
    rt.pointer_up(x + 60.0, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.settings.quality, Quality::Max);
    let Ok(EngineCmd::ApplySettings(sent)) = rx.try_recv() else {
        panic!("settings were not sent");
    };
    assert_eq!(sent.quality, Quality::Max);
    assert!(!sent.first_run_done);
}

#[test]
fn the_test_recording_runs_then_says_it_works_with_the_clip() {
    let (mut rt, rx) = first_run_runtime();
    tap(&mut rt, "first-run-test");
    assert_eq!(
        rx.try_recv(),
        Ok(EngineCmd::StartTestRecording { secs: 10 })
    );
    assert_eq!(rt.state.first_test, TestRun::Running);
    settle(&mut rt);
    assert!(rt.find("first-run-testing").is_some());
    assert!(rt.find("first-run-test").is_some());
    assert!(rt.find("first-run-test-spinner").is_some());
    let mut entry = Ui::preview().recordings.remove(0);
    entry.meta.duration_secs = 10.0;
    rt.update(move |ui, tx| ui.apply(UiMsg::RecordingSaved(entry), tx));
    settle(&mut rt);
    assert!(rt.find("first-run-passed").is_some());
    assert!(has_text(&rt, "It works"));
    assert!(rt.find("first-run-watch").is_some());
}

#[test]
fn a_failed_test_shows_the_error_and_lets_the_user_try_again() {
    let (mut rt, _rx) = first_run_runtime();
    tap(&mut rt, "first-run-test");
    rt.update(|ui, tx| ui.apply(UiMsg::Status(EngineStatus::Error(NO_FRAMES.into())), tx));
    settle(&mut rt);
    assert_eq!(rt.state.first_test, TestRun::Failed);
    assert!(rt.find("first-run-failed").is_some());
    assert!(has_text(&rt, "League is in Fullscreen"));
    rt.update(|ui, tx| ui.apply(UiMsg::Status(EngineStatus::Idle), tx));
    settle(&mut rt);
    assert!(rt.find("first-run-test").is_some());
}

#[test]
fn a_recording_that_is_not_the_test_does_not_say_it_works() {
    let (mut rt, _rx) = first_run_runtime();
    let entry = Ui::preview().recordings.remove(0);
    rt.update(move |ui, tx| ui.apply(UiMsg::RecordingSaved(entry), tx));
    settle(&mut rt);
    assert!(rt.find("first-run-passed").is_none());
}
