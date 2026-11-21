use super::*;

#[test]
fn the_capture_crates_own_messages_are_told_apart() {
    let cases = [
        (NO_FRAMES, Problem::ExclusiveFullscreen),
        (
            "no frames from the game window: if League runs in exclusive fullscreen, switch it",
            Problem::ExclusiveFullscreen,
        ),
        (
            "screen capture failed to start: windows graphics capture is not supported",
            Problem::WindowsTooOld,
        ),
        (
            "video encoder failed: no usable hardware H.264 encoder: MFT refused",
            Problem::NoEncoder,
        ),
        ("no usable GPU: adapter missing", Problem::NoEncoder),
        (
            "the captured window closed during startup",
            Problem::LeagueNotFound,
        ),
        (
            "screen capture failed to start: windows api error: Invalid window handle. (0x80070578)",
            Problem::LeagueNotFound,
        ),
        (
            "screen capture failed to start: windows api error: Access is denied. (0x80070005)",
            Problem::CaptureBlocked,
        ),
        ("capture setup timed out", Problem::Other),
        ("encoder stopped producing frames", Problem::Other),
    ];
    for (raw, expected) in cases {
        assert_eq!(Problem::from_error(raw), expected, "{raw}");
    }
}

#[test]
fn folder_and_disk_errors_are_told_apart_and_disk_wins() {
    assert_eq!(
        Problem::from_error(&format!("{FOLDER_UNUSABLE}: D:\\Games: Access is denied")),
        Problem::FolderUnusable
    );
    assert_eq!(
        Problem::from_error("There is not enough space on the disk. (os error 112)"),
        Problem::LowDisk
    );
    assert_eq!(
        Problem::from_error(&format!(
            "{FOLDER_UNUSABLE}: No space left on device (os error 28)"
        )),
        Problem::LowDisk
    );
}

#[test]
fn every_problem_has_unique_anchor_and_plain_text() {
    let mut anchors: Vec<&str> = Problem::ALL.iter().map(|p| p.anchor()).collect();
    anchors.sort_unstable();
    anchors.dedup();
    assert_eq!(anchors.len(), Problem::ALL.len());
    for problem in Problem::ALL {
        assert!(!problem.title().is_empty() && !problem.action().is_empty());
        assert!(problem.status_title().len() <= 20, "{:?}", problem);
        assert!(problem.status_detail().len() <= 24, "{:?}", problem);
        assert!(problem.help_url().starts_with(FAQ_URL));
    }
}

#[test]
fn only_a_failure_that_time_cannot_fix_skips_the_retries() {
    assert!(Problem::ExclusiveFullscreen.retry_helps());
    assert!(Problem::Other.retry_helps());
    assert!(!Problem::FolderUnusable.retry_helps());
    assert!(!Problem::NoEncoder.retry_helps());
}

#[test]
fn a_notice_is_plain_for_a_known_problem_and_raw_for_an_unknown_one() {
    assert_eq!(
        Problem::notice("Capture failed", NO_FRAMES),
        "League is in Fullscreen. Switch League to Borderless in its video settings, then retry."
    );
    assert_eq!(
        Problem::notice("Capture failed", "odd thing"),
        "Capture failed: odd thing"
    );
}
