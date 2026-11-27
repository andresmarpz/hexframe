use super::*;

#[test]
fn a_folder_that_refuses_writes_is_an_unusable_folder() {
    for cause in [
        "Permission denied (os error 13)",
        "Access is denied. (os error 5)",
        "The system cannot find the path specified. (os error 3)",
        "Not a directory (os error 20)",
    ] {
        assert_eq!(
            Problem::from_error(&format!("{FOLDER_UNUSABLE}: D:\\Games: {cause}")),
            Problem::FolderUnusable,
            "{cause}"
        );
    }
}

#[test]
fn a_full_disk_is_told_from_an_access_error() {
    for raw in [
        "There is not enough space on the disk. (os error 112)",
        "No space left on device (os error 28)",
        "The disk is full",
    ] {
        assert_eq!(Problem::from_error(raw), Problem::LowDisk, "{raw}");
    }
    assert_eq!(
        Problem::from_error("Access is denied. (0x80070005)"),
        Problem::CaptureBlocked
    );
}

#[test]
fn a_refused_write_is_a_folder_problem_not_a_capture_block() {
    for raw in [
        "Access is denied. (os error 5)",
        "rename failed: Access is denied. (os error 5)",
    ] {
        assert_eq!(Problem::from_error(raw), Problem::FolderUnusable, "{raw}");
    }
}

#[test]
fn a_notice_for_an_unusable_folder_says_to_choose_another() {
    let raw = format!("{FOLDER_UNUSABLE}: D:\\Games: Access is denied");
    assert_eq!(
        Problem::notice("Recording failed", &raw),
        "Can't save to the recordings folder. Choose another recordings folder."
    );
    assert!(Problem::FolderUnusable.offers_folder());
    assert!(Problem::LowDisk.offers_folder());
}

#[test]
fn a_full_disk_write_error_gets_its_own_text() {
    assert_eq!(
        Problem::write_failure_notice("io error: no storage space"),
        DISK_FULL_NOTICE
    );
    assert_eq!(
        Problem::write_failure_notice("io error: No space left on device (os error 28)"),
        DISK_FULL_NOTICE
    );
    assert_eq!(
        Problem::write_failure_notice("io error: broken pipe"),
        WRITE_ERROR_NOTICE
    );
}
