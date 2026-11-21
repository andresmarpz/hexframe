use super::*;

#[test]
fn the_start_refusal_names_the_free_space_and_links_to_the_help_entry() {
    let UiMsg::NoticeWithAction {
        level,
        text,
        action,
    } = refused(1_450_000_000)
    else {
        panic!("not a notice with an action");
    };

    assert_eq!(level, NoticeLevel::Error);
    assert_eq!(
        text,
        "Not enough disk space to record (1.4 GB free). Free up space or change the recordings folder."
    );
    assert_eq!(
        action,
        NoticeAction::Link("https://greyscreen-lake.vercel.app/faq#low-disk".to_owned())
    );
}

#[test]
fn the_mid_recording_stop_names_the_free_space_and_links_to_the_help_entry() {
    let UiMsg::NoticeWithAction { text, action, .. } = stopped(400_000_000) else {
        panic!("not a notice with an action");
    };

    assert_eq!(
        text,
        "Recording stopped: the disk is almost full (0.4 GB free). Free up space or change the recordings folder."
    );
    assert_eq!(action, NoticeAction::Link(LOW_DISK_URL.to_owned()));
}

#[test]
fn the_system_provider_reads_a_folder_that_does_not_exist_yet_from_its_parent() {
    let dir = std::env::temp_dir().join("greyscreen-missing-folder-for-free-space/a/b");

    assert!(
        SystemFreeSpace
            .free_bytes(&dir)
            .is_some_and(|free| free > 0)
    );
}

#[test]
fn the_default_check_interval_is_ten_seconds() {
    assert_eq!(SystemFreeSpace.check_every(), Duration::from_secs(10));
}
