use std::collections::HashSet;

use super::unique_name;

#[test]
fn a_free_name_is_kept() {
    assert_eq!(unique_name("00-05_Kill.mp4", |_| false), "00-05_Kill.mp4");
}

#[test]
fn a_taken_name_gets_the_next_free_number_and_stays_mp4() {
    let mut taken: HashSet<String> = HashSet::new();
    let mut names = Vec::new();
    for _ in 0..4 {
        let name = unique_name("00-05_Kill.mp4", |c| taken.contains(c));
        assert!(name.ends_with(".mp4"));
        assert!(taken.insert(name.clone()), "{name} was reused");
        names.push(name);
    }
    assert_eq!(
        names,
        [
            "00-05_Kill.mp4",
            "00-05_Kill_2.mp4",
            "00-05_Kill_3.mp4",
            "00-05_Kill_4.mp4"
        ]
    );
}

#[test]
fn a_gap_in_the_numbers_is_filled_without_touching_taken_names() {
    let taken: HashSet<&str> = ["a.mp4", "a_2.mp4", "a_4.mp4"].into();
    assert_eq!(unique_name("a.mp4", |c| taken.contains(c)), "a_3.mp4");
}
