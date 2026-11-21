use serde_json::json;

use super::*;

fn tree() -> Vec<Element> {
    Element::from_tree(&json!({"elements": [
        {"path": "kit-root/nav/nav-recent/nav-game-1", "key": "nav-game-1", "label": "Lux, Victory", "rect": [0, 0, 10, 10], "opacity": 1.0},
        {"path": "kit-root/page/games-list/row-1", "key": "row-1", "label": "Lux, ARAM, Victory", "rect": [100, 50, 200, 40], "opacity": 1.0},
        {"path": "kit-root/page/games-list/row-2/title", "text": "Jinx", "rect": [100, 90, 200, 40], "opacity": 1.0},
        {"path": "kit-root/page/fading", "key": "fading", "rect": [0, 0, 10, 10], "opacity": 0.0, "focused": true},
    ]}))
}

#[test]
fn a_plain_name_matches_a_key_or_a_path_suffix() {
    let els = tree();
    assert_eq!(
        Locator::parse("row-1").find(&els).map(|e| e.rect[1]),
        Some(50.0)
    );
    assert!(Locator::parse("row-2/title").find(&els).is_some());
    assert!(Locator::parse("ow-1").find(&els).is_none());
}

#[test]
fn a_scope_picks_the_match_inside_it() {
    let els = tree();
    let row = Locator::parse("games-list >> label:Lux")
        .find(&els)
        .expect("row");
    assert_eq!(row.key.as_deref(), Some("row-1"));
    assert_eq!(row.centre(), (200.0, 70.0));
    let first = Locator::parse("label:Lux").find(&els).expect("any");
    assert_eq!(first.key.as_deref(), Some("nav-game-1"));
}

#[test]
fn text_locators_read_the_text() {
    assert!(Locator::parse("text:Jinx").find(&tree()).is_some());
    assert!(Locator::parse("text:Lux").find(&tree()).is_none());
}

#[test]
fn invisible_elements_are_not_found() {
    assert!(Locator::parse("fading").find(&tree()).is_none());
}

#[test]
fn only_unscoped_keys_go_to_rune_as_keys() {
    assert_eq!(Locator::parse("nav-games").plain_key(), Some("nav-games"));
    assert_eq!(Locator::parse("label:Games").plain_key(), None);
    assert_eq!(Locator::parse("nav >> nav-games").plain_key(), None);
}
