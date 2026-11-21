use rune::TextEdit;

use super::*;

fn library() -> Vec<RecordingEntry> {
    Ui::preview().recordings
}

fn champions(shown: &[&RecordingEntry]) -> Vec<String> {
    shown
        .iter()
        .map(|rec| rec.meta.game.info.me.champion.clone())
        .collect()
}

#[test]
fn an_untouched_filter_shows_everything() {
    let all = library();
    let filter = RecordingsFilter::default();
    assert!(!filter.is_active());
    assert_eq!(filter.apply(&all).len(), all.len());
}

#[test]
fn champion_search_is_a_case_insensitive_substring() {
    let all = library();
    let mut filter = RecordingsFilter {
        champion: TextEdit::new("  aHr "),
        ..RecordingsFilter::default()
    };
    assert!(filter.is_active());
    assert_eq!(champions(&filter.apply(&all)), ["Ahri", "Ahri", "Ahri"]);
    filter.champion = TextEdit::new("in");
    assert_eq!(champions(&filter.apply(&all)), ["Jinx"]);
    filter.champion = TextEdit::new("nobody");
    assert!(filter.apply(&all).is_empty());
}

#[test]
fn search_also_matches_the_mode_and_the_result() {
    let all = library();
    let search = |text: &str| RecordingsFilter {
        champion: TextEdit::new(text),
        ..RecordingsFilter::default()
    };
    assert_eq!(champions(&search("aram").apply(&all)), ["Yasuo", "Ahri"]);
    assert_eq!(search("victory").apply(&all).len(), 4);
    assert_eq!(search("defeat").apply(&all).len(), 3);
}

#[test]
fn clearing_resets_the_search() {
    let mut filter = RecordingsFilter {
        champion: TextEdit::new("ahri"),
        pending_delete: None,
    };
    assert!(filter.is_active());
    filter.clear();
    assert!(!filter.is_active());
}
