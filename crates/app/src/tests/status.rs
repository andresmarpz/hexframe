use greyscreen_engine::{EngineCmd, EngineStatus};

use super::{live_runtime, tap};
use crate::ui::Ui;

fn recording_for(rt: &mut rune::Runtime<Ui>, secs: u32) {
    rt.update(|ui, _| {
        ui.status = Ui::preview().status;
        ui.elapsed_secs = secs;
    });
    rt.frame(1.0 / 60.0);
}

fn show(rt: &mut rune::Runtime<Ui>, status: EngineStatus) {
    rt.update(|ui, _| ui.status = status);
    rt.frame(1.0 / 60.0);
}

fn failed() -> EngineStatus {
    EngineStatus::Error("capture did not start".into())
}

#[test]
fn stop_after_a_minute_asks_for_a_second_click() {
    let (mut rt, rx) = live_runtime();
    recording_for(&mut rt, 754);
    tap(&mut rt, "stop-recording");
    assert!(rx.try_recv().is_err());
    assert!(rt.state.stop_confirm_until.is_some());
    tap(&mut rt, "stop-recording");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::StopRecording));
    assert!(rt.state.stop_confirm_until.is_none());
}

#[test]
fn stop_confirmation_runs_out_after_three_seconds() {
    let (mut rt, rx) = live_runtime();
    recording_for(&mut rt, 754);
    tap(&mut rt, "stop-recording");
    rt.update(|ui, tx| {
        assert!(ui.poll(100.0, tx));
    });
    rt.frame(1.0 / 30.0);
    assert!(rt.state.stop_confirm_until.is_none());
    tap(&mut rt, "stop-recording");
    assert!(rx.try_recv().is_err());
}

#[test]
fn stop_in_the_first_minute_needs_one_click() {
    let (mut rt, rx) = live_runtime();
    recording_for(&mut rt, 60);
    tap(&mut rt, "stop-recording");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::StopRecording));
}

#[test]
fn leaving_the_recording_clears_a_pending_stop_confirmation() {
    let (mut rt, _rx) = live_runtime();
    recording_for(&mut rt, 754);
    tap(&mut rt, "stop-recording");
    assert!(rt.state.stop_confirm_until.is_some());
    rt.update(|ui, tx| {
        ui.apply(greyscreen_engine::UiMsg::Status(EngineStatus::Finalizing), tx);
    });
    assert!(rt.state.stop_confirm_until.is_none());
}

#[test]
fn retry_in_the_error_state_asks_the_engine_to_try_again() {
    let (mut rt, rx) = live_runtime();
    show(&mut rt, failed());
    assert!(rt.find("error-message").is_some());
    tap(&mut rt, "retry-capture");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::Retry));
}

#[test]
fn dismissing_the_error_returns_to_waiting() {
    let (mut rt, rx) = live_runtime();
    show(&mut rt, failed());
    tap(&mut rt, "dismiss-error");
    assert_eq!(rt.state.status, EngineStatus::Idle);
    assert!(rx.try_recv().is_err());
    assert!(rt.find("retry-capture").is_none());
    assert!(rt.find("dismiss-error").is_none());
}

#[test]
fn error_actions_show_only_in_the_error_state() {
    let (mut rt, _rx) = live_runtime();
    assert!(rt.find("retry-capture").is_none());
    assert!(rt.find("dismiss-error").is_none());
    show(&mut rt, failed());
    assert!(rt.find("retry-capture").is_some());
    assert!(rt.find("error-open-logs").is_some());
    assert!(rt.find("test-recording").is_none());
}

#[test]
fn a_disabled_test_recording_button_says_why() {
    let (mut rt, _rx) = live_runtime();
    assert!(rt.find("test-recording-reason").is_none());
    show(&mut rt, EngineStatus::Waiting);
    assert!(rt.find("test-recording-reason").is_some());
    show(&mut rt, EngineStatus::Finalizing);
    assert!(rt.find("test-recording-reason").is_some());
}

#[test]
fn the_last_game_card_shows_the_result_and_kda() {
    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| ui.last_game = ui.recordings.first().cloned());
    rt.frame(1.0 / 60.0);
    assert!(rt.find("last-game-result").is_some());
    assert!(rt.find("last-game-kda").is_some());
    assert!(rt.find("open-last-game").is_some());
}
