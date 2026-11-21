use greyscreen_engine::EngineStatus;
use rune::{Key, Runtime};

use super::{press, press_char, runtime, settle, tap};
use crate::ui::death_entry;
use crate::ui::{Detail, Ui};

fn browsing() -> Runtime<Ui> {
    let mut rt = runtime();
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| {
        ui.status = EngineStatus::Idle;
        ui.show_welcome();
        ui.recordings[0] = death_entry();
    });
    settle(&mut rt);
    rt
}

fn first_game(rt: &Runtime<Ui>) -> String {
    let at = rt.state.visible_games()[0];
    rt.state.recordings[at].meta.id.clone()
}

fn right_click(rt: &mut Runtime<Ui>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.secondary_down(x, y);
    rt.secondary_up(x, y);
    settle(rt);
}

#[test]
fn the_welcome_pane_shows_recent_clips_and_this_week() {
    let rt = browsing();
    assert_eq!(rt.state.detail(), Detail::Welcome);
    assert!(rt.find("recent-clips").is_some());
    assert!(rt.find("this-week").is_some());
}

#[test]
fn a_clip_on_the_welcome_pane_opens_its_game_and_plays_it() {
    let mut rt = browsing();
    let at = rt.state.recordings[1].meta.id.clone();
    rt.update(|ui, _| {
        ui.recordings.swap(0, 1);
    });
    settle(&mut rt);
    let key = format!("home-clip-{at}-0");
    tap(&mut rt, &key);
    assert_eq!(rt.state.detail(), Detail::Game);
    assert_eq!(rt.state.selected_id(), Some(at.as_str()));
}

#[test]
fn right_click_on_a_row_offers_review_deaths_and_opens_the_mode() {
    let mut rt = browsing();
    let id = first_game(&rt);
    right_click(&mut rt, &id);
    assert!(rt.state.row_menu().is_some());
    assert!(rt.find("row-menu-menu").is_some());
    tap(&mut rt, "row-menu-item1");
    assert!(rt.state.row_menu().is_none());
    assert_eq!(rt.state.detail(), Detail::Game);
    let review = rt.state.review.as_ref().unwrap();
    assert!(review.death_active());
}

#[test]
fn a_game_without_deaths_has_no_review_deaths_item() {
    let mut rt = browsing();
    rt.update(|ui, _| ui.recordings[0].meta.events.clear());
    let id = first_game(&rt);
    right_click(&mut rt, &id);
    assert!(rt.find("row-menu-menu").is_some());
    assert!(rt.find("row-menu-item2").is_none());
}

#[test]
fn escape_closes_the_row_menu_without_opening_the_game() {
    let mut rt = browsing();
    let id = first_game(&rt);
    right_click(&mut rt, &id);
    press(&mut rt, Key::Escape);
    assert!(rt.state.row_menu().is_none());
    assert_eq!(rt.state.detail(), Detail::Welcome);
}

#[test]
fn the_delete_item_asks_before_removing() {
    let mut rt = browsing();
    let id = first_game(&rt);
    right_click(&mut rt, &id);
    tap(&mut rt, "row-menu-item4");
    assert_eq!(rt.state.filter.pending_delete.as_deref(), Some(id.as_str()));
}

#[test]
fn the_review_deaths_button_hides_while_the_mode_runs() {
    let mut rt = browsing();
    let id = first_game(&rt);
    tap(&mut rt, &id);
    assert!(rt.find("review-deaths").is_some());
    tap(&mut rt, "review-deaths");
    assert!(rt.find("review-deaths").is_none());
    assert!(rt.find("death-review").is_some());
}

#[test]
fn up_and_down_still_move_between_games_in_the_death_review() {
    let mut rt = browsing();
    let id = first_game(&rt);
    tap(&mut rt, &id);
    press_char(&mut rt, "d");
    assert!(rt.state.review.as_ref().unwrap().death_active());
    press(&mut rt, Key::Down);
    assert_eq!(rt.state.detail(), Detail::Game);
    assert_ne!(rt.state.selected_id(), Some(id.as_str()));
    assert!(!rt.state.review.as_ref().unwrap().death_active());
}

#[test]
fn a_game_row_shows_how_many_deaths_are_reviewed() {
    let mut rt = browsing();
    let has = |rt: &Runtime<Ui>, wanted: &str| {
        super::visible_texts(rt)
            .iter()
            .any(|t| t.starts_with("Ahri") && t.ends_with(wanted))
    };
    assert!(has(&rt, "0 of 7 reviewed"));
    rt.update(|ui, _| ui.recordings[0].meta.review.set_note(2, "late"));
    settle(&mut rt);
    assert!(has(&rt, "1 of 7 reviewed"));
}
