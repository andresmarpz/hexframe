use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn plain_runs_are_not_qa() {
    assert_eq!(parse(args(&["--snap", "a.png"])), Mode::default());
}

#[test]
fn the_qa_flag_turns_on_the_fixed_clock() {
    assert!(parse(args(&["--qa"])).active);
}

#[test]
fn a_library_implies_qa_and_names_the_folder() {
    let mode = parse(args(&["--scene", "games-pick", "--qa-library", "/tmp/lib"]));
    assert!(mode.active);
    assert_eq!(mode.library, Some(PathBuf::from("/tmp/lib")));
}

#[test]
fn the_fixed_clock_parses() {
    let now: Zoned = FIXED_NOW.parse().expect("fixed now");
    assert_eq!(now.year(), 2026);
}
