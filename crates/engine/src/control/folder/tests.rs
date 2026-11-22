use greyscreen_core::problem::Problem;

use super::*;

#[test]
fn a_missing_folder_is_made_and_leaves_no_probe_behind() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("deep").join("Greyscreen");
    assert!(check_writable(&dir).is_ok());
    assert!(dir.is_dir());
    assert_eq!(fs::read_dir(&dir).expect("read").count(), 0);
}

#[test]
fn a_path_that_is_a_file_is_an_unusable_folder() {
    let root = tempfile::tempdir().expect("tempdir");
    let file = root.path().join("not-a-folder");
    fs::write(&file, b"x").expect("write");
    let err = check_writable(&file).expect_err("a file is not a folder");
    assert_eq!(Problem::from_error(&err), Problem::FolderUnusable);
    assert!(err.contains("not-a-folder"));
}

#[cfg(unix)]
#[test]
fn a_read_only_folder_is_an_unusable_folder() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("locked");
    fs::create_dir(&dir).expect("dir");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).expect("chmod");
    let result = check_writable(&dir);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("chmod back");
    if let Err(err) = result {
        assert_eq!(Problem::from_error(&err), Problem::FolderUnusable);
    }
}

#[test]
fn an_empty_or_relative_folder_is_an_unusable_folder() {
    for dir in ["", "recordings", "./recordings", "../up"] {
        let err = check_writable(Path::new(dir)).expect_err("not absolute");
        assert_eq!(Problem::from_error(&err), Problem::FolderUnusable, "{dir}");
    }
}
