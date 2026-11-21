use crossbeam_channel::{Receiver, unbounded};
use rune::{Env, Runtime};

use super::state::{Cmd, Msg, Phase, SetupState};
use super::{scenes, view};

fn state() -> (SetupState, Receiver<Cmd>) {
    let (tx, rx) = unbounded();
    (SetupState::new("C:\\p".into(), Some(tx)), rx)
}

fn runtime() -> (Runtime<SetupState>, Receiver<Cmd>) {
    let (state, rx) = state();
    let mut rt = Runtime::new(state, view::root, (460.0, 420.0), Env::new(true, 1.0));
    rt.frame(0.0);
    (rt, rx)
}

fn tap(rt: &mut Runtime<SetupState>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

#[test]
fn starts_ready_with_start_with_windows_on() {
    let (state, _) = state();
    assert_eq!(state.phase, Phase::Ready);
    assert!(state.start_with_windows);
}

#[test]
fn install_asks_the_worker_with_the_chosen_option() {
    let (mut state, rx) = state();
    state.toggle_start_with_windows();
    state.install();
    assert_eq!(state.phase, Phase::Copying);
    assert_eq!(
        rx.try_recv(),
        Ok(Cmd::Install {
            start_with_windows: false
        })
    );
}

#[test]
fn a_second_install_press_while_busy_does_nothing() {
    let (mut state, rx) = state();
    state.install();
    state.install();
    assert_eq!(rx.try_iter().count(), 1);
}

#[test]
fn option_is_locked_while_installing() {
    let (mut state, _) = state();
    state.install();
    state.toggle_start_with_windows();
    assert!(state.start_with_windows);
}

#[test]
fn closing_a_running_copy_comes_before_copying_and_done() {
    let (mut state, _) = state();
    state.install();
    state.apply(Msg::Closing);
    assert_eq!(state.phase, Phase::Closing);
    state.apply(Msg::Copying);
    assert_eq!(state.phase, Phase::Copying);
    state.apply(Msg::Done);
    assert_eq!(state.phase, Phase::Done);
}

#[test]
fn failure_keeps_the_reason_and_install_can_retry() {
    let (mut state, rx) = state();
    state.install();
    state.apply(Msg::Failed("still running".into()));
    assert_eq!(state.phase, Phase::Failed("still running".into()));
    rx.try_iter().for_each(drop);
    state.install();
    assert_eq!(state.phase, Phase::Copying);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Install { .. })));
}

#[test]
fn late_progress_does_not_leave_the_done_screen() {
    let (mut state, _) = state();
    state.install();
    state.apply(Msg::Done);
    state.apply(Msg::Copying);
    assert_eq!(state.phase, Phase::Done);
}

#[test]
fn open_works_only_after_done() {
    let (mut state, rx) = state();
    state.open_installed();
    state.install();
    state.open_installed();
    rx.try_iter().for_each(drop);
    state.apply(Msg::Done);
    state.open_installed();
    assert_eq!(rx.try_recv(), Ok(Cmd::OpenInstalled));
}

#[test]
fn run_here_works_only_after_a_failure() {
    let (mut state, rx) = state();
    state.run_here();
    assert!(rx.try_recv().is_err());
    state.apply(Msg::Failed("x".into()));
    state.run_here();
    assert_eq!(rx.try_recv(), Ok(Cmd::RunHere));
}

#[test]
fn every_scene_builds_a_state() {
    for (name, scene) in scenes::SCENES {
        let mut state = scenes::preview();
        scene(&mut state);
        assert!(name.starts_with("setup"), "{name}");
    }
}

#[test]
fn install_button_starts_the_install() {
    let (mut rt, rx) = runtime();
    tap(&mut rt, "setup-install");
    assert_eq!(rt.state.phase, Phase::Copying);
    assert_eq!(
        rx.try_recv(),
        Ok(Cmd::Install {
            start_with_windows: true
        })
    );
    assert!(rt.find("setup-install").is_none());
}

#[test]
fn checkbox_flips_the_start_with_windows_choice() {
    let (mut rt, _) = runtime();
    tap(&mut rt, "setup-start-with-windows");
    assert!(!rt.state.start_with_windows);
}

#[test]
fn done_screen_offers_open_and_error_screen_offers_the_fallback() {
    let (mut rt, rx) = runtime();
    rt.update(|s, _| s.phase = Phase::Done);
    rt.frame(1.0 / 60.0);
    tap(&mut rt, "setup-open");
    assert_eq!(rx.try_recv(), Ok(Cmd::OpenInstalled));

    rt.update(|s, _| s.phase = Phase::Failed("nope".into()));
    rt.frame(1.0 / 60.0);
    assert!(rt.find("setup-error").is_some());
    tap(&mut rt, "setup-run-here");
    assert_eq!(rx.try_recv(), Ok(Cmd::RunHere));
}
