use super::*;

#[test]
fn the_windows_build_comes_from_the_ver_line() {
    assert_eq!(
        windows_build("\r\nMicrosoft Windows [Version 10.0.22631.4460]\r\n").as_deref(),
        Some("10.0.22631.4460")
    );
    for line in [
        "Microsoft Windows [Versión 10.0.22631.4460]",
        "Microsoft Windows [version 10.0.22631.4460]",
        "Microsoft Windows [Versão 10.0.22631.4460]",
        "Microsoft Windows [10.0.22631.4460]",
    ] {
        assert_eq!(windows_build(line).as_deref(), Some("10.0.22631.4460"));
    }
    assert_eq!(windows_build("Microsoft Windows []"), None);
    assert_eq!(windows_build("nothing useful"), None);
    assert_eq!(windows_build(""), None);
}

#[test]
fn errors_read_as_plain_sentences() {
    let unreachable = FeedbackError::Request {
        url: "https://x".into(),
        reason: "dns".into(),
    };
    assert!(describe(&unreachable).starts_with("Couldn't reach Greyscreen"));
    assert!(describe(&FeedbackError::Status { status: 503 }).contains("503"));
}

#[test]
fn the_real_os_string_is_not_empty() {
    assert!(!system_os().trim().is_empty());
}
