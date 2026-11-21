use super::{nav_rect_of, runtime, settle, tap, visible_texts};
use crate::ui::library::format::recent_games;
use crate::ui::{Detail, Screen};

fn newest_ids(rt: &rune::Runtime<crate::ui::Ui>) -> Vec<String> {
    recent_games(&rt.state.recordings, 5)
        .iter()
        .map(|rec| rec.meta.id.clone())
        .collect()
}

#[test]
fn recent_lists_the_five_newest_games_top_down() {
    let rt = runtime();
    let ids = newest_ids(&rt);
    assert_eq!(ids.len(), 5);
    let tops: Vec<f32> = ids
        .iter()
        .map(|id| nav_rect_of(&rt, &format!("nav-game-{id}"))[1])
        .collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    assert!(rt.state.recordings.len() > 5);
}

#[test]
fn recent_is_left_out_without_games() {
    let mut rt = runtime();
    rt.update(|ui, _| {
        ui.close_game();
        ui.recordings.clear();
    });
    settle(&mut rt);
    assert!(rt.find("nav-recent").is_none());
}

#[test]
fn clicking_a_recent_game_opens_it_in_games() {
    let mut rt = runtime();
    rt.update(|ui, _| {
        ui.close_game();
        ui.screen = Screen::Home;
    });
    settle(&mut rt);
    let ids = newest_ids(&rt);
    tap(&mut rt, &format!("nav-game-{}", ids[2]));
    assert_eq!(rt.state.screen, Screen::Games);
    assert_eq!(rt.state.detail(), Detail::Game);
    assert_eq!(rt.state.selected_id(), Some(ids[2].as_str()));
    tap(&mut rt, &format!("nav-game-{}", ids[0]));
    assert_eq!(rt.state.selected_id(), Some(ids[0].as_str()));
}

#[test]
fn the_storage_block_names_the_size_and_the_limit() {
    let mut rt = runtime();
    assert!(rt.find("nav-storage").is_some());
    assert!(rt.find("nav-storage-meter").is_none());
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.starts_with("Storage: ") && t.ends_with(" used") && !t.contains(" of "))
    );
    rt.update(|ui, _| ui.settings.storage_limit_gb = Some(50));
    settle(&mut rt);
    assert!(rt.find("nav-storage-meter").is_some());
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.starts_with("Storage: ") && t.ends_with("of 50 GB used"))
    );
}
