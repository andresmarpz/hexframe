use greyscreen_core::settings::Settings;
use greyscreen_engine::{ClipEntry, EngineStatus, FeedItem, NoticeLevel, RecordingEntry, UiMsg};
use rune::Tx;

use super::{FEED_CAP, Ui};

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
