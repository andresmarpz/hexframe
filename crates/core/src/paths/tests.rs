use super::*;
use std::path::Path;

#[test]
fn settings_file_lives_in_config_dir() {
    let file = settings_file();
    assert_eq!(file.parent(), Some(config_dir().as_path()));
    assert_eq!(
        file.file_name().and_then(|n| n.to_str()),
        Some("settings.json")
    );
}

#[test]
fn directories_are_named_after_the_app() {
    assert!(config_dir().ends_with("Greyscreen"));
    assert!(logs_dir().ends_with(Path::new("Greyscreen").join("logs")));
    assert!(default_output_dir().ends_with("Greyscreen"));
}
