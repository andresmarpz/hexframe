use greyscreen_engine::{EngineCmd, EngineStatus};

use super::{live_runtime, tap};
use crate::ui::{Screen, Ui};

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

fn idle_runtime() -> (rune::Runtime<Ui>, crossbeam_channel::Receiver<EngineCmd>) {
    let (mut rt, rx) = live_runtime();
    show(&mut rt, EngineStatus::Idle);
    (rt, rx)
}

#[test]
fn home_is_the_first_screen_and_shows_the_live_card() {
    let (rt, _rx) = idle_runtime();
    assert_eq!(rt.state.screen, Screen::Home);
    assert!(rt.find("state-card").is_some());
    assert!(rt.find("record-games").is_some());
}

#[test]
fn home_shows_the_library_sections_with_demo_data() {
    let (rt, _rx) = idle_runtime();
    for key in [
        "recent-games",
        "this-week",
        "recent-clips",
        "view-all-games",
    ] {
        assert!(rt.find(key).is_some(), "missing {key}");
    }
    assert!(rt.find("welcome").is_none());
    assert!(rt.find("feed-card").is_none());
}

#[test]
fn recent_games_lists_the_five_newest_recordings() {
    let (rt, _rx) = idle_runtime();
    for rec in rt.state.recordings.iter().take(5) {
        assert!(rt.find(&format!("home-game-{}", rec.meta.id)).is_some());
    }
    let sixth = &rt.state.recordings[5].meta.id;
    assert!(rt.find(&format!("home-game-{sixth}")).is_none());
}

#[test]
fn clicking_a_recent_game_opens_review() {
    let (mut rt, _rx) = idle_runtime();
    let id = rt.state.recordings[2].meta.id.clone();
    tap(&mut rt, &format!("home-game-{id}"));
    assert_eq!(rt.state.screen, Screen::Review);
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.entry.meta.id.clone()),
        Some(id)
    );
}

#[test]
fn view_all_goes_to_recordings() {
    let (mut rt, rx) = idle_runtime();
    tap(&mut rt, "view-all-games");
    assert_eq!(rt.state.screen, Screen::Recordings);
    assert_eq!(rx.try_recv(), Ok(EngineCmd::RescanLibrary));
}

#[test]
fn clicking_a_recent_clip_opens_that_game_playing_the_clip() {
    let (mut rt, _rx) = idle_runtime();
    let clips = crate::ui::library_format::recent_clips(&rt.state.recordings, 6);
    let first = clips[0];
    let (id, index) = (first.entry.meta.id.clone(), first.index);
    rt.wheel(700.0, 300.0, -600.0);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    tap(&mut rt, &format!("home-clip-{id}-{index}"));
    assert_eq!(rt.state.screen, Screen::Review);
    let review = rt.state.review.as_ref().expect("review");
    assert_eq!(review.entry.meta.id, id);
    assert_eq!(review.active_clip(), Some(index));
}

#[test]
fn an_empty_library_shows_the_welcome_card_and_no_library_sections() {
    let (mut rt, rx) = idle_runtime();
    rt.update(|ui, _| ui.recordings.clear());
    rt.frame(1.0 / 60.0);
    assert!(rt.find("welcome").is_some());
    for key in ["recent-games", "this-week", "recent-clips"] {
        assert!(rt.find(key).is_none(), "{key} should be hidden");
    }
    tap(&mut rt, "welcome-test-recording");
    assert_eq!(
        rx.try_recv(),
        Ok(EngineCmd::StartTestRecording { secs: 10 })
    );
}

#[test]
fn recording_shows_the_feed_under_the_live_card_and_no_library() {
    let (mut rt, _rx) = idle_runtime();
    recording_for(&mut rt, 90);
    assert!(rt.find("feed-card").is_some());
    for key in ["recent-games", "this-week", "recent-clips", "welcome"] {
        assert!(rt.find(key).is_none(), "{key} should be hidden");
    }
}

#[test]
fn the_recordings_page_no_longer_shows_the_output_folder() {
    let (mut rt, _rx) = idle_runtime();
    tap(&mut rt, "nav-recordings");
    let folder = rt.state.settings.output_dir.display().to_string();
    assert!(
        !super::visible_texts(&rt)
            .iter()
            .any(|t| t.contains(&folder))
    );
    assert!(rt.find("open-folder").is_some());
    assert!(rt.find("recordings-summary").is_some());
}
