use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    ClipEntry, EngineStatus, FeedItem, NoticeAction, NoticeLevel, RecordingEntry, UiMsg,
};
use rune::Tx;

use super::review::jump::{next_target, previous_target};
use super::{FEED_CAP, Ui, format_size_rate};

fn ui() -> Ui {
    Ui::blank(Settings::default())
}

fn recording() -> EngineStatus {
    EngineStatus::Recording {
        since: 0,
        game: None,
    }
}

fn feed_item(id: u32) -> FeedItem {
    FeedItem {
        id,
        rec_secs: 1.0,
        game_secs: 1.0,
        text: "x".into(),
        highlights: Vec::new(),
    }
}

fn entry(index: usize) -> RecordingEntry {
    Ui::preview().recordings.remove(index)
}

#[test]
fn starting_a_recording_clears_the_feed_and_starts_the_clock() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(UiMsg::Feed(feed_item(1)), &mut tx);
    ui.apply(UiMsg::Status(recording()), &mut tx);
    assert!(ui.feed.is_empty());
    assert_eq!(ui.recording_since_clock, Some(0.0));
}

#[test]
fn a_repeated_recording_status_keeps_the_feed() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(UiMsg::Status(recording()), &mut tx);
    ui.apply(UiMsg::Feed(feed_item(1)), &mut tx);
    ui.apply(UiMsg::Status(recording()), &mut tx);
    assert_eq!(ui.feed.len(), 1);
}

#[test]
fn leaving_recording_stops_the_clock_and_keeps_the_feed() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(UiMsg::Status(recording()), &mut tx);
    ui.apply(UiMsg::Feed(feed_item(1)), &mut tx);
    ui.apply(UiMsg::Status(EngineStatus::Finalizing), &mut tx);
    assert_eq!(ui.recording_since_clock, None);
    assert_eq!(ui.feed.len(), 1);
}

#[test]
fn returning_to_idle_clears_the_feed() {
    for idle in [EngineStatus::Idle, EngineStatus::GameRunning] {
        let mut ui = ui();
        let mut tx = Tx::default();
        ui.apply(UiMsg::Status(recording()), &mut tx);
        ui.apply(UiMsg::Feed(feed_item(1)), &mut tx);
        ui.apply(UiMsg::Status(idle), &mut tx);
        assert!(ui.feed.is_empty());
    }
}

#[test]
fn a_saved_recording_becomes_the_last_game_until_the_next_one_starts() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(UiMsg::RecordingSaved(entry(0)), &mut tx);
    assert_eq!(ui.last_game, Some(entry(0)));
    ui.apply(UiMsg::Status(EngineStatus::Waiting), &mut tx);
    assert!(ui.last_game.is_some());
    ui.apply(UiMsg::Status(recording()), &mut tx);
    assert!(ui.last_game.is_none());
}

#[test]
fn the_feed_keeps_only_the_newest_items() {
    let mut ui = ui();
    let mut tx = Tx::default();
    for id in 0..(FEED_CAP as u32 + 20) {
        ui.apply(UiMsg::Feed(feed_item(id)), &mut tx);
    }
    assert_eq!(ui.feed.len(), FEED_CAP);
    assert_eq!(ui.feed.front().map(|f| f.id), Some(20));
}

#[test]
fn a_saved_recording_replaces_the_row_with_the_same_id() {
    let mut ui = ui();
    let mut tx = Tx::default();
    let mut first = entry(0);
    ui.apply(UiMsg::RecordingSaved(first.clone()), &mut tx);
    first.size_bytes += 1;
    ui.apply(UiMsg::RecordingSaved(first.clone()), &mut tx);
    ui.apply(UiMsg::RecordingSaved(entry(1)), &mut tx);
    assert_eq!(ui.recordings.len(), 2);
    assert_eq!(ui.recordings[1].size_bytes, first.size_bytes);
    assert!(ui.notice.is_some());
}

#[test]
fn a_saved_clip_shows_a_notice_with_its_label() {
    let mut ui = ui();
    let mut tx = Tx::default();
    let clip = entry(0).meta.clips.remove(0);
    let saved = ClipEntry {
        clip,
        folder: "x".into(),
    };
    ui.apply(UiMsg::ClipSaved(saved), &mut tx);
    let (notice, _) = ui.notice.expect("notice");
    assert_eq!(notice.text, "Clip saved: MyKill");
}

#[test]
fn a_notice_disappears_after_four_seconds() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(
        UiMsg::Notice {
            level: NoticeLevel::Warn,
            text: "careful".into(),
        },
        &mut tx,
    );
    assert!(!ui.poll(3.9, &mut tx));
    assert!(ui.notice.is_some());
    assert!(ui.poll(4.0, &mut tx));
    assert!(ui.notice.is_none());
}

#[test]
fn a_notice_with_an_action_stays_long_enough_to_press_it() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(
        UiMsg::NoticeWithAction {
            level: NoticeLevel::Error,
            text: "Not enough disk space to record (1.4 GB free).".into(),
            action: NoticeAction::Link("https://example.com".into()),
        },
        &mut tx,
    );
    assert!(!ui.poll(4.0, &mut tx));
    let (notice, _) = ui.notice.clone().expect("notice");
    assert_eq!(
        notice.action,
        Some(NoticeAction::Link("https://example.com".into()))
    );
    assert!(ui.poll(12.0, &mut tx));
    assert!(ui.notice.is_none());
}

#[test]
fn a_plain_notice_has_no_action() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(
        UiMsg::Notice {
            level: NoticeLevel::Info,
            text: "Recovered 1 recording that was cut off.".into(),
        },
        &mut tx,
    );
    assert_eq!(ui.notice.expect("notice").0.action, None);
}

#[test]
fn poll_reports_a_change_only_when_the_second_changes() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(UiMsg::Status(recording()), &mut tx);
    assert!(!ui.poll(0.5, &mut tx));
    assert!(ui.poll(1.1, &mut tx));
    assert_eq!(ui.elapsed_secs, 1);
    assert!(!ui.poll(1.6, &mut tx));
}

#[test]
fn a_library_message_replaces_the_recordings() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(UiMsg::Library(vec![entry(0)]), &mut tx);
    assert_eq!(ui.recordings.len(), 1);
}

fn event_at(rec_secs: f64) -> greyscreen_core::library::EventEntry {
    let mut event = entry(0).meta.events[0].clone();
    event.rec_secs = rec_secs;
    event
}

fn events() -> Vec<greyscreen_core::library::EventEntry> {
    [30.0, 10.0, 20.0].map(event_at).to_vec()
}

#[test]
fn next_event_is_the_first_landing_spot_after_now() {
    assert_eq!(next_target(&events(), 0.0), Some(5.0));
    assert_eq!(next_target(&events(), 5.0), Some(15.0));
    assert_eq!(next_target(&events(), 14.0), Some(15.0));
    assert_eq!(next_target(&events(), 15.0), Some(25.0));
    assert_eq!(next_target(&events(), 25.0), None);
    assert_eq!(next_target(&[], 0.0), None);
}

#[test]
fn previous_event_is_the_last_landing_spot_before_now() {
    assert_eq!(previous_target(&events(), 100.0), Some(25.0));
    assert_eq!(previous_target(&events(), 25.0), Some(15.0));
    assert_eq!(previous_target(&events(), 15.5), Some(5.0));
    assert_eq!(previous_target(&events(), 5.0), None);
    assert_eq!(previous_target(&[], 5.0), None);
}

#[test]
fn events_in_the_first_seconds_land_at_zero() {
    let early = [1.0, 2.0].map(event_at).to_vec();
    assert_eq!(next_target(&early, 0.0), None);
    assert_eq!(previous_target(&early, 4.0), Some(0.0));
}

#[test]
fn size_rate_rounds_and_marks_native() {
    assert_eq!(
        format_size_rate(63.4, None),
        "About 65 MB per minute at these settings."
    );
    assert_eq!(
        format_size_rate(412.0, Some(1440)),
        "About 410 MB per minute at these settings (1440p)."
    );
}
