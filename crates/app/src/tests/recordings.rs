use greyscreen_engine::{EngineCmd, UiMsg};
use rune::Key;

use super::{live_runtime, press_char, runtime_with, tap};
use crate::ui::recordings_filter::BIN;
use crate::ui::{ResultFilter, Screen, Ui};

fn on_recordings() -> (rune::Runtime<Ui>, crossbeam_channel::Receiver<EngineCmd>) {
    let (mut rt, rx) = live_runtime();
    tap(&mut rt, "nav-recordings");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::RescanLibrary));
    (rt, rx)
}

fn id_of(rt: &rune::Runtime<Ui>, champion: &str) -> String {
    rt.state
        .recordings
        .iter()
        .find(|r| r.meta.game.info.me.champion == champion)
        .map(|r| r.meta.id.clone())
        .unwrap_or_else(|| panic!("no {champion} recording"))
}

fn type_text(rt: &mut rune::Runtime<Ui>, text: &str) {
    tap(rt, "filter-champion");
    for c in text.chars() {
        press_char(rt, &c.to_string());
    }
}

#[test]
fn the_empty_library_shows_a_state_and_no_table_header() {
    let (cmd, _rx) = crossbeam_channel::unbounded();
    let mut ui = Ui::live(greyscreen_core::settings::Settings::default(), cmd);
    ui.screen = Screen::Recordings;
    let mut rt = runtime_with(ui);
    assert!(rt.find("recordings-empty").is_some());
    assert!(rt.find("recordings-header").is_none());
    assert!(rt.find("recordings-toolbar").is_none());
    assert!(rt.find("recordings-summary").is_none());
    tap(&mut rt, "go-to-status");
    assert_eq!(rt.state.screen, Screen::Status);
}

#[test]
fn typing_a_champion_narrows_the_rows() {
    let (mut rt, _rx) = on_recordings();
    let ahri = id_of(&rt, "Ahri");
    let yasuo = id_of(&rt, "Yasuo");
    assert!(rt.find(&ahri).is_some());
    assert!(rt.find(&yasuo).is_some());
    type_text(&mut rt, "YAS");
    assert_eq!(rt.state.filter.champion.text, "YAS");
    assert!(rt.find(&yasuo).is_some());
    assert!(rt.find(&ahri).is_none());
}

#[test]
fn the_result_and_mode_filters_narrow_the_rows_and_the_summary() {
    let (mut rt, _rx) = on_recordings();
    rt.update(|ui, _| {
        ui.filter.result = ResultFilter::Defeat;
        ui.filter.mode = Some("Summoner's Rift".into());
    });
    rt.frame(1.0 / 60.0);
    let thresh = id_of(&rt, "Thresh");
    let yasuo = id_of(&rt, "Yasuo");
    assert!(rt.find(&thresh).is_some());
    assert!(rt.find(&yasuo).is_none());
    assert!(rt.find("recordings-summary").is_some());
}

#[test]
fn the_result_control_and_mode_select_drive_the_filter() {
    let (mut rt, _rx) = on_recordings();
    tap(&mut rt, "filter-mode-trigger");
    assert!(rt.state.filter.mode_open);
    tap(&mut rt, "option2");
    assert_eq!(rt.state.filter.mode.as_deref(), Some("Summoner's Rift"));
    assert!(!rt.state.filter.mode_open);
    tap(&mut rt, "segment1");
    assert_eq!(rt.state.filter.result, ResultFilter::Victory);
}

#[test]
fn an_empty_result_offers_to_clear_the_filters() {
    let (mut rt, _rx) = on_recordings();
    type_text(&mut rt, "zzz");
    assert!(rt.find("recordings-no-match").is_some());
    assert!(rt.find("recordings-header").is_none());
    tap(&mut rt, "clear-filters");
    assert!(!rt.state.filter.is_active());
    assert!(rt.find("recordings-no-match").is_none());
    assert!(rt.find(&id_of(&rt, "Ahri")).is_some());
}

#[test]
fn deleting_asks_first_and_cancel_keeps_the_recording() {
    let (mut rt, rx) = on_recordings();
    let id = id_of(&rt, "Ahri");
    tap(&mut rt, &format!("delete-{id}"));
    assert_eq!(rt.state.filter.pending_delete.as_deref(), Some(id.as_str()));
    assert_eq!(rt.state.screen, Screen::Recordings);
    assert!(rt.find("delete-popover").is_some());
    tap(&mut rt, "delete-cancel");
    assert!(rt.state.filter.pending_delete.is_none());
    assert!(rt.find("delete-popover").is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn pressing_outside_or_escape_dismisses_the_confirmation() {
    let (mut rt, rx) = on_recordings();
    let id = id_of(&rt, "Ahri");
    tap(&mut rt, &format!("delete-{id}"));
    super::press(&mut rt, Key::Escape);
    assert!(rt.state.filter.pending_delete.is_none());
    tap(&mut rt, &format!("delete-{id}"));
    tap(&mut rt, "delete-scrim");
    assert!(rt.state.filter.pending_delete.is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn confirming_sends_the_delete_and_the_new_library_drops_the_row() {
    let (mut rt, rx) = on_recordings();
    let id = id_of(&rt, "Ahri");
    let before = rt.state.recordings.len();
    tap(&mut rt, &format!("delete-{id}"));
    tap(&mut rt, "delete-confirm");
    assert_eq!(
        rx.try_recv(),
        Ok(EngineCmd::DeleteRecording { id: id.clone() })
    );
    assert!(rt.state.filter.pending_delete.is_none());
    let remaining: Vec<_> = rt
        .state
        .recordings
        .iter()
        .filter(|r| r.meta.id != id)
        .cloned()
        .collect();
    rt.update(|ui, tx| ui.apply(UiMsg::Library(remaining), tx));
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.recordings.len(), before - 1);
    assert!(rt.find(&id).is_none());
}

#[test]
fn the_confirmation_names_the_size_the_champion_and_the_bin() {
    let (mut rt, _rx) = on_recordings();
    let id = id_of(&rt, "Ahri");
    tap(&mut rt, &format!("delete-{id}"));
    let entry = rt.state.pending_delete_entry().expect("pending").clone();
    assert_eq!(entry.meta.id, id);
    assert!(BIN == "Trash" || BIN == "Recycle Bin");
}

#[test]
fn review_does_not_offer_delete() {
    let (mut rt, _rx) = on_recordings();
    let id = id_of(&rt, "Ahri");
    tap(&mut rt, &id);
    assert_eq!(rt.state.screen, Screen::Review);
    assert!(rt.find(&format!("delete-{id}")).is_none());
}
