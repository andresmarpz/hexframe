use greyscreen_engine::EngineStatus;
use rune::Runtime;

use super::{nav_rect_of, runtime, settle, tap, visible_texts};
use crate::ui::{Screen, Ui};

fn recording_runtime(screen: Screen) -> Runtime<Ui> {
    let mut rt = runtime();
    rt.update(move |ui, _| ui.screen = screen);
    settle(&mut rt);
    rt
}

#[test]
fn the_pill_shows_the_elapsed_time_on_recordings_while_recording() {
    let rt = recording_runtime(Screen::Recordings);
    assert!(rt.find("nav-recording").is_some());
    assert!(visible_texts(&rt).iter().any(|t| t.contains("12:34")));
}

#[test]
fn the_pill_sits_next_to_the_status_tab() {
    let rt = recording_runtime(Screen::Settings);
    let status = nav_rect_of(&rt, "nav-status");
    let pill = nav_rect_of(&rt, "nav-recording");
    let recordings = nav_rect_of(&rt, "nav-recordings");
    assert!(pill[0] >= status[0] + status[2]);
    assert!(pill[0] + pill[2] <= recordings[0]);
}

#[test]
fn the_pill_is_hidden_on_status() {
    let rt = recording_runtime(Screen::Status);
    assert!(rt.find("nav-recording").is_none());
}

#[test]
fn the_pill_is_hidden_when_not_recording() {
    let mut rt = recording_runtime(Screen::Recordings);
    rt.update(|ui, _| ui.status = EngineStatus::Idle);
    rt.frame(1.0 / 60.0);
    assert!(rt.find("nav-recording").is_none());
}

#[test]
fn the_pill_follows_the_clock() {
    let mut rt = recording_runtime(Screen::Recordings);
    rt.update(|ui, _| ui.elapsed_secs = 61);
    rt.frame(1.0 / 60.0);
    assert!(visible_texts(&rt).iter().any(|t| t.contains("01:01")));
}

#[test]
fn clicking_the_pill_goes_to_status() {
    let mut rt = recording_runtime(Screen::Settings);
    tap(&mut rt, "nav-recording");
    assert_eq!(rt.state.screen, Screen::Status);
    assert!(rt.find("nav-recording").is_none());
}

#[test]
fn the_record_switch_is_called_record_games_and_explained() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    let texts = visible_texts(&rt);
    assert!(texts.iter().any(|t| t == "Record games"));
    assert!(
        texts
            .iter()
            .any(|t| t == "Start recording when a League game starts.")
    );
    assert!(!texts.iter().any(|t| t == "Record automatically"));
}

#[test]
fn the_audio_switch_says_what_is_recorded() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    assert!(
        visible_texts(&rt).iter().any(
            |t| t == "Records League's own audio. Discord and your microphone aren't recorded."
        )
    );
}

#[test]
fn the_size_estimate_ends_with_a_period() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    let line = visible_texts(&rt)
        .into_iter()
        .find(|t| t.contains("MB per minute"))
        .expect("size estimate");
    assert!(line.ends_with('.'), "{line}");
}

#[test]
fn the_about_card_carries_the_riot_notice() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    assert!(rt.find("riot-notice").is_some());
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.starts_with("Greyscreen isn't endorsed by Riot Games")
                && t.ends_with("registered trademarks of Riot Games, Inc."))
    );
}
