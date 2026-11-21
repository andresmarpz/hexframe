use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::Runtime;

use super::{live_runtime, runtime, settle, tap, visible_texts};
use crate::ui::{Screen, Ui};

fn recording_on(screen: Screen) -> Runtime<Ui> {
    let mut rt = runtime();
    rt.update(move |ui, _| ui.screen = screen);
    settle(&mut rt);
    rt
}

#[test]
fn the_status_block_shows_the_elapsed_time_on_every_screen() {
    for screen in [Screen::Home, Screen::Games, Screen::Clips] {
        let rt = recording_on(screen);
        assert!(rt.find("nav-status").is_some());
        assert!(visible_texts(&rt).iter().any(|t| t.contains("12:34")));
    }
}

#[test]
fn the_status_block_follows_the_clock() {
    let mut rt = recording_on(Screen::Games);
    rt.update(|ui, _| ui.elapsed_secs = 61);
    rt.frame(1.0 / 60.0);
    assert!(visible_texts(&rt).iter().any(|t| t.contains("01:01")));
}

#[test]
fn the_status_block_names_each_recorder_state() {
    let mut rt = runtime();
    for (status, expected) in [
        (EngineStatus::Idle, "Ready"),
        (EngineStatus::GameRunning, "Not recording"),
        (EngineStatus::Waiting, "League is running"),
        (EngineStatus::Finalizing, "Saving"),
        (EngineStatus::Error("x".into()), "Capture failed"),
    ] {
        rt.update(move |ui, _| ui.status = status.clone());
        settle(&mut rt);
        assert!(
            visible_texts(&rt).iter().any(|t| t.contains(expected)),
            "{expected}"
        );
    }
    rt.update(|ui, _| {
        ui.status = EngineStatus::Idle;
        ui.settings.auto_record = false;
    });
    settle(&mut rt);
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.contains("Recording off"))
    );
}

#[test]
fn clicking_the_status_block_goes_home() {
    let mut rt = recording_on(Screen::Clips);
    tap(&mut rt, "nav-status");
    assert_eq!(rt.state.screen, Screen::Home);
}

#[test]
fn save_clip_sits_in_the_status_block_only_while_recording() {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| {
        ui.status = Ui::preview().status;
        ui.screen = Screen::Games;
    });
    settle(&mut rt);
    tap(&mut rt, "nav-save-clip");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::SaveClipNow));
    rt.update(|ui, _| ui.status = EngineStatus::Idle);
    settle(&mut rt);
    assert!(rt.find("nav-save-clip").is_none());
}

#[test]
fn the_games_list_header_keeps_only_the_search() {
    let rt = recording_on(Screen::Games);
    assert!(rt.find("filter-champion").is_some());
    assert!(rt.find("nav-status-label").is_none());
}
