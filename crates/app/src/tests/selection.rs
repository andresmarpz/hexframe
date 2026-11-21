use greyscreen_engine::EngineStatus;
use rune::{Key, Runtime};

use super::{live_runtime, runtime, settle, tap};
use crate::ui::{Detail, ListItem, Ui};

fn game_ids(rt: &Runtime<Ui>) -> Vec<String> {
    rt.state
        .visible_games()
        .into_iter()
        .map(|i| rt.state.recordings[i].meta.id.clone())
        .collect()
}

fn idle(rt: &mut Runtime<Ui>) {
    rt.update(|ui, _| {
        ui.status = EngineStatus::Idle;
        ui.close_game();
    });
    settle(rt);
}

#[test]
fn the_list_groups_games_under_day_headings_newest_first() {
    let mut rt = runtime();
    idle(&mut rt);
    let items = rt.state.list_items();
    assert!(matches!(&items[0], ListItem::Day(day) if day == "Today"));
    let days: Vec<_> = items
        .iter()
        .filter_map(|item| match item {
            ListItem::Day(day) => Some(day.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(days[..2], ["Today", "Yesterday"]);
    let games = items
        .iter()
        .filter(|item| matches!(item, ListItem::Game(_)))
        .count();
    assert_eq!(games, rt.state.recordings.len());
}

#[test]
fn a_recording_adds_a_live_row_on_top_that_selects_the_live_pane() {
    let mut rt = runtime();
    assert_eq!(rt.state.list_items()[0], ListItem::Live);
    assert_eq!(rt.state.detail(), Detail::Live);
    let first = game_ids(&rt)[0].clone();
    tap(&mut rt, &first);
    assert_eq!(rt.state.detail(), Detail::Game);
    tap(&mut rt, "live-row");
    assert_eq!(rt.state.detail(), Detail::Live);
    assert!(rt.state.review.is_none());
    assert!(rt.find("feed-card").is_some());
}

#[test]
fn the_live_row_goes_away_when_the_game_is_saved() {
    let mut rt = runtime();
    rt.update(|ui, _| ui.status = EngineStatus::Idle);
    rt.update(|ui, _| {
        ui.live_selected = false;
    });
    settle(&mut rt);
    assert!(rt.find("live-row").is_none());
    assert_eq!(rt.state.detail(), Detail::Pick);
}

#[test]
fn arrow_keys_step_through_games_without_wrapping() {
    let mut rt = runtime();
    idle(&mut rt);
    rt.on_key = Some(Ui::on_key);
    let ids = game_ids(&rt);
    super::press(&mut rt, Key::Down);
    assert_eq!(rt.state.selected_id(), Some(ids[0].as_str()));
    super::press(&mut rt, Key::Down);
    assert_eq!(rt.state.selected_id(), Some(ids[1].as_str()));
    super::press(&mut rt, Key::Up);
    super::press(&mut rt, Key::Up);
    assert_eq!(rt.state.selected_id(), Some(ids[0].as_str()));
    for _ in 0..ids.len() + 2 {
        super::press(&mut rt, Key::Down);
    }
    assert_eq!(rt.state.selected_id(), Some(ids[ids.len() - 1].as_str()));
}

#[test]
fn arrow_up_from_nothing_selects_the_last_game_and_the_live_row_comes_first() {
    let mut rt = runtime();
    rt.on_key = Some(Ui::on_key);
    super::press(&mut rt, Key::Down);
    assert_eq!(rt.state.detail(), Detail::Game);
    super::press(&mut rt, Key::Up);
    assert_eq!(rt.state.detail(), Detail::Live);
}

#[test]
fn searching_a_champion_filters_the_list_and_clearing_restores_it() {
    let mut rt = runtime();
    idle(&mut rt);
    rt.update(|ui, _| ui.filter.champion = rune::TextEdit::new("yasuo"));
    settle(&mut rt);
    assert_eq!(game_ids(&rt).len(), 1);
    rt.update(|ui, _| ui.filter.champion = rune::TextEdit::new("zzz"));
    settle(&mut rt);
    assert!(rt.find("games-empty").is_some());
    tap(&mut rt, "clear-filters");
    assert_eq!(game_ids(&rt).len(), rt.state.recordings.len());
}

#[test]
fn enter_moves_focus_to_the_player() {
    let mut rt = super::key_review_runtime();
    super::press(&mut rt, Key::Enter);
    let focused = rt
        .elements()
        .into_iter()
        .find(|e| e.focused)
        .and_then(|e| e.key);
    assert_eq!(focused.as_deref(), Some("review-video"));
}

#[test]
fn an_empty_library_shows_a_quiet_list_and_the_first_run_card_on_home() {
    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| {
        ui.recordings.clear();
        ui.settings.first_run_done = false;
    });
    tap(&mut rt, "nav-games");
    assert!(rt.find("games-empty").is_some());
    assert!(rt.find("games-pick").is_some());
    tap(&mut rt, "nav-home");
    assert!(rt.find("first-run").is_some());
}

#[test]
fn deleting_the_open_game_returns_to_the_placeholder() {
    let mut rt = runtime();
    idle(&mut rt);
    let id = game_ids(&rt)[0].clone();
    tap(&mut rt, &id);
    let rest: Vec<_> = rt.state.recordings[1..].to_vec();
    rt.update(move |ui, _| ui.apply_library(rest));
    settle(&mut rt);
    assert_eq!(rt.state.detail(), Detail::Pick);
}
