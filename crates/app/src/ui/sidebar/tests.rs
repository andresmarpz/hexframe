use super::*;
use crate::ui::Ui;

#[test]
fn totals_add_up_the_size_of_every_game() {
    let recordings = Ui::preview().recordings;
    let totals = LibraryTotals::of(&recordings);
    let expected: u64 = recordings.iter().map(|rec| rec.size_bytes).sum();
    assert_eq!(totals.bytes, expected);
    assert_eq!(LibraryTotals::of(&[]), LibraryTotals::default());
}

#[test]
fn the_used_fraction_needs_a_limit_and_stops_at_full() {
    let totals = LibraryTotals {
        bytes: 5_000_000_000,
    };
    assert_eq!(totals.used_fraction(None), None);
    assert_eq!(totals.used_fraction(Some(0)), None);
    assert_eq!(totals.used_fraction(Some(10_000_000_000)), Some(0.5));
    assert_eq!(totals.used_fraction(Some(2_000_000_000)), Some(1.0));
}
