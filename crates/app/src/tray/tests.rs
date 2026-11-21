use greyscreen_core::problem::NO_FRAMES;
use greyscreen_core::time::HNS_PER_SEC;
use greyscreen_engine::EngineStatus;
use rune::TrayItem;

use super::view::{SAVE_ID, TOGGLE_ID, View};
use super::{Link, TrayState};

fn state(status: EngineStatus, auto_record: bool) -> TrayState {
    TrayState {
        status,
        auto_record,
    }
}

fn recording(since_secs: i64) -> EngineStatus {
    EngineStatus::Recording {
        since: since_secs * HNS_PER_SEC,
        game: None,
    }
}

fn label_of(view: &View, id: &str) -> (String, bool) {
    view.items
        .iter()
        .find_map(|item| match item {
            TrayItem::Action {
                id: found,
                label,
                enabled,
            } if found == id => Some((label.clone(), *enabled)),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn the_tooltip_says_what_the_recorder_is_doing() {
    let now = 754 * HNS_PER_SEC;
    let tip = |status, auto| View::of(&state(status, auto), now).tooltip;
    assert_eq!(tip(recording(0), true), "Greyscreen — Recording 12:34");
    assert_eq!(
        tip(EngineStatus::Idle, true),
        "Greyscreen — Waiting for League"
    );
    assert_eq!(
        tip(EngineStatus::Idle, false),
        "Greyscreen — Recording paused"
    );
    assert_eq!(
        tip(EngineStatus::GameRunning, false),
        "Greyscreen — Recording paused"
    );
    assert_eq!(
        tip(EngineStatus::Waiting, true),
        "Greyscreen — League is loading"
    );
    assert_eq!(tip(EngineStatus::Finalizing, true), "Greyscreen — Saving");
    assert_eq!(
        tip(EngineStatus::Error(NO_FRAMES.into()), true),
        "Greyscreen — League is in Fullscreen"
    );
}

#[test]
fn the_pause_item_mirrors_the_record_games_switch() {
    let on = View::of(&state(EngineStatus::Idle, true), 0);
    let off = View::of(&state(EngineStatus::Idle, false), 0);
    assert_eq!(label_of(&on, TOGGLE_ID).0, "Pause recording");
    assert_eq!(label_of(&off, TOGGLE_ID).0, "Resume recording");
}

#[test]
fn save_clip_is_enabled_only_while_recording() {
    for (status, enabled) in [
        (EngineStatus::Idle, false),
        (EngineStatus::Waiting, false),
        (EngineStatus::Finalizing, false),
        (recording(0), true),
    ] {
        let view = View::of(&state(status.clone(), true), 0);
        assert_eq!(
            label_of(&view, SAVE_ID),
            ("Save clip".to_owned(), enabled),
            "{status:?}"
        );
    }
}

#[test]
fn the_icon_switches_to_the_recording_variant_only_while_recording() {
    assert!(View::of(&state(recording(0), true), 0).recording);
    assert!(!View::of(&state(EngineStatus::Finalizing, true), 0).recording);
    assert!(!View::of(&state(EngineStatus::Idle, true), 0).recording);
}

#[test]
fn the_menu_has_open_first_and_quit_last() {
    let view = View::of(&state(EngineStatus::Idle, true), 0);
    let labels: Vec<_> = view
        .items
        .iter()
        .filter_map(|item| match item {
            TrayItem::Action { label, .. } => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        labels,
        ["Open Greyscreen", "Pause recording", "Save clip", "Quit"]
    );
}

#[test]
fn a_link_hands_the_latest_state_to_the_tray() {
    let (link, rx) = Link::detached();
    link.send(state(EngineStatus::Waiting, true));
    assert_eq!(rx.try_recv().map(|s| s.status), Ok(EngineStatus::Waiting));
}
