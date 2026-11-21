use rune::{Runtime, TextEdit};

use super::{runtime, settle, tap};
use crate::ui::{Screen, Ui};

fn clear_button_in(rt: &Runtime<Ui>, field: &str) -> bool {
    rt.elements()
        .into_iter()
        .any(|e| e.key.as_deref() == Some("action") && e.path.split('/').any(|part| part == field))
}

#[test]
fn the_games_search_shows_clear_only_with_text() {
    let mut rt = runtime();
    rt.update(|ui, _| ui.go(Screen::Games));
    settle(&mut rt);
    assert!(rt.find("filter-champion").is_some());
    assert!(!clear_button_in(&rt, "filter-champion"));
    rt.update(|ui, _| ui.filter.champion = TextEdit::new("ah"));
    settle(&mut rt);
    assert!(clear_button_in(&rt, "filter-champion"));
    rt.update(|ui, _| ui.filter.champion = TextEdit::default());
    settle(&mut rt);
    assert!(!clear_button_in(&rt, "filter-champion"));
}

#[test]
fn the_settings_search_shows_clear_only_with_text() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    settle(&mut rt);
    assert!(!clear_button_in(&rt, "settings-search"));
    rt.update(|ui, _| ui.settings_search = TextEdit::new("qual"));
    settle(&mut rt);
    assert!(clear_button_in(&rt, "settings-search"));
}
