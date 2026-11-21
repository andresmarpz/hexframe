use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::Runtime;

use super::{live_runtime, settle, tap};
use crate::ui::{Ui, death_entry};
use crossbeam_channel::Receiver;

fn browsing() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| {
        ui.status = EngineStatus::Idle;
        ui.go(crate::ui::Screen::Games);
        ui.close_game();
        ui.recordings[0] = death_entry();
    });
    settle(&mut rt);
    (rt, rx)
}

fn first_game(rt: &Runtime<Ui>) -> String {
    let at = rt.state.visible_games()[0];
    rt.state.recordings[at].meta.id.clone()
}

fn ask_from_row_menu(rt: &mut Runtime<Ui>, id: &str) {
    let (x, y) = rt.find(id).expect("game row");
    rt.secondary_down(x, y);
    rt.secondary_up(x, y);
    settle(rt);
    tap(rt, "row-menu-item4");
    settle(rt);
}

fn deletes(rx: &Receiver<EngineCmd>) -> Vec<String> {
    rx.try_iter()
        .filter_map(|cmd| match cmd {
            EngineCmd::DeleteRecording { id } => Some(id),
            _ => None,
        })
        .collect()
}

#[test]
fn the_row_menu_opens_a_centered_dialog_and_deletes_on_confirm() {
    let (mut rt, rx) = browsing();
    let id = first_game(&rt);
    ask_from_row_menu(&mut rt, &id);
    assert!(rt.find("delete-dialog").is_some());
    let panel = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("delete-dialog"))
        .expect("dialog panel");
    let (centre_x, centre_y) = (
        panel.rect[0] + panel.rect[2] / 2.0,
        panel.rect[1] + panel.rect[3] / 2.0,
    );
    assert!((centre_x - 640.0).abs() < 2.0, "x {centre_x}");
    assert!((centre_y - 360.0).abs() < 40.0, "y {centre_y}");
    assert!(deletes(&rx).is_empty());
    tap(&mut rt, "delete-confirm");
    settle(&mut rt);
    assert_eq!(deletes(&rx), vec![id]);
    assert!(rt.find("delete-dialog").is_none());
}

#[test]
fn cancel_keeps_the_game() {
    let (mut rt, rx) = browsing();
    let id = first_game(&rt);
    ask_from_row_menu(&mut rt, &id);
    tap(&mut rt, "delete-cancel");
    settle(&mut rt);
    assert!(rt.find("delete-dialog").is_none());
    assert!(deletes(&rx).is_empty());
    assert!(rt.state.recordings.iter().any(|r| r.meta.id == id));
}

#[test]
fn the_dialog_names_the_champion_and_the_size() {
    let (mut rt, _rx) = browsing();
    let id = first_game(&rt);
    ask_from_row_menu(&mut rt, &id);
    let texts = super::visible_texts(&rt);
    assert!(texts.iter().any(|t| t == "Delete this game?"));
    assert!(texts.iter().any(|t| t.starts_with("Ahri · ")));
}

#[test]
fn the_toolbar_trash_button_opens_the_same_dialog() {
    let (mut rt, rx) = browsing();
    let id = first_game(&rt);
    tap(&mut rt, &id);
    settle(&mut rt);
    tap(&mut rt, &format!("delete-{id}"));
    settle(&mut rt);
    assert!(rt.find("delete-dialog").is_some());
    tap(&mut rt, "delete-confirm");
    assert_eq!(deletes(&rx), vec![id]);
}

#[test]
fn deleting_the_open_game_closes_its_player_first() {
    let (mut rt, rx) = browsing();
    rt.update(|ui, _| ui.player_factory = crate::ui::fake_player::factory);
    let id = first_game(&rt);
    tap(&mut rt, &id);
    settle(&mut rt);
    assert!(rt.state.review.is_some());
    assert_eq!(crate::ui::fake_player::open_players(), 1);
    tap(&mut rt, &format!("delete-{id}"));
    settle(&mut rt);
    tap(&mut rt, "delete-confirm");
    assert!(rt.state.review.is_none());
    assert_eq!(crate::ui::fake_player::open_players(), 0);
    assert_eq!(deletes(&rx), vec![id]);
}
