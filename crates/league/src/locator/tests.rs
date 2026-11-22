use std::fs;
use std::path::{Path, PathBuf};

use super::rule::{Candidate, is_game_window, text_from_wide};

const GAME_PATH: &str = r"C:\Riot Games\League of Legends\Game\League of Legends.exe";
const OWN_PID: u32 = 100;

fn matches(class: &str, image_path: &str, pid: u32) -> bool {
    let candidate = Candidate {
        class,
        image_path,
        pid,
    };
    is_game_window(&candidate, OWN_PID)
}

#[test]
fn the_league_window_matches() {
    assert!(matches("RiotWindowClass", GAME_PATH, 7));
}

#[test]
fn a_decoy_of_the_riot_class_from_another_program_is_ignored() {
    assert!(!matches("RiotWindowClass", r"C:\Tools\decoy.exe", 8));
}

#[test]
fn the_class_must_match_exactly() {
    assert!(!matches("RiotWindowClassX", GAME_PATH, 7));
    assert!(!matches("Other", GAME_PATH, 7));
}

#[test]
fn our_own_process_and_pid_zero_are_skipped() {
    assert!(!matches("RiotWindowClass", GAME_PATH, OWN_PID));
    assert!(!matches("RiotWindowClass", GAME_PATH, 0));
}

#[test]
fn a_long_executable_path_still_matches_by_file_name() {
    let path = format!(r"C:\{}\League of Legends.exe", "a".repeat(300));
    assert!(matches("RiotWindowClass", &path, 7));
}

#[test]
fn the_file_name_match_ignores_case() {
    assert!(matches(
        "RiotWindowClass",
        r"D:\games\LEAGUE OF LEGENDS.EXE",
        7
    ));
}

#[test]
fn a_directory_named_like_the_game_does_not_match() {
    assert!(!matches(
        "RiotWindowClass",
        r"C:\League of Legends.exe\other.exe",
        7
    ));
}

#[test]
fn wide_text_clamps_a_reported_length_past_the_buffer() {
    let buffer: Vec<u16> = "abc".encode_utf16().collect();
    assert_eq!(text_from_wide(&buffer, 1000), "abc");
    assert_eq!(text_from_wide(&buffer, 2), "ab");
    assert_eq!(text_from_wide(&buffer, 0), "");
}

#[test]
fn wide_text_with_broken_surrogates_does_not_panic() {
    let buffer = [0xD800u16, 0x41];
    assert_eq!(text_from_wide(&buffer, 2), "\u{FFFD}A");
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_game_process_is_opened_only_in_the_locator_and_with_limited_rights() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut files = Vec::new();
    for krate in fs::read_dir(crates).unwrap() {
        let src = krate.unwrap().path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    let needle = ["Open", "Process("].concat();
    let mut openers = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for line in text.lines().filter(|l| l.contains(&needle)) {
            openers.push((file.clone(), line.to_string()));
        }
    }
    assert!(!openers.is_empty(), "locator no longer opens the process");
    for (file, line) in openers {
        assert!(
            file.ends_with("league/src/locator/win.rs"),
            "OpenProcess outside the locator: {}",
            file.display()
        );
        assert!(line.contains("PROCESS_QUERY_LIMITED_INFORMATION"));
    }
}
