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
fn result_filter_keeps_victories_or_defeats_only() {
    let all = library();
    let mut filter = RecordingsFilter {
        result: ResultFilter::Victory,
        ..RecordingsFilter::default()
    };
    let wins = filter.apply(&all);
    assert_eq!(wins.len(), 4);
    assert!(wins.iter().all(|r| r.meta.game.result == GameResult::Win));
    filter.result = ResultFilter::Defeat;
    let losses = filter.apply(&all);
    assert_eq!(losses.len(), 3);
    assert!(
        losses
            .iter()
            .all(|r| r.meta.game.result == GameResult::Loss)
    );
}

#[test]
fn mode_filter_matches_the_humanized_mode() {
    let all = library();
    let filter = RecordingsFilter {
        mode: Some("ARAM".into()),
        ..RecordingsFilter::default()
    };
    assert_eq!(champions(&filter.apply(&all)), ["Yasuo", "Ahri"]);
}

#[test]
fn filters_combine() {
    let all = library();
    let filter = RecordingsFilter {
        champion: TextEdit::new("ahri"),
        result: ResultFilter::Victory,
        mode: Some("Summoner's Rift".into()),
        ..RecordingsFilter::default()
    };
    assert_eq!(filter.apply(&all).len(), 1);
}

#[test]
fn modes_present_are_humanized_sorted_and_unique() {
    assert_eq!(modes_present(&library()), ["ARAM", "Summoner's Rift"]);
    assert!(modes_present(&[]).is_empty());
}

#[test]
fn clearing_resets_every_filter() {
    let mut filter = RecordingsFilter {
        champion: TextEdit::new("ahri"),
        result: ResultFilter::Defeat,
        mode: Some("ARAM".into()),
        mode_open: true,
        pending_delete: None,
    };
    assert!(filter.is_active());
    filter.clear();
    assert!(!filter.is_active());
    assert!(!filter.mode_open);
}
