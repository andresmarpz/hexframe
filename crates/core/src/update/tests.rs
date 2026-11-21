use super::*;

fn v(text: &str) -> Version {
    Version::parse(text).unwrap_or_else(|| panic!("cannot parse {text}"))
}

fn release(tag: &str, prerelease: bool) -> Release {
    Release {
        tag: tag.into(),
        prerelease,
        published: None,
        exe: Some(Asset {
            url: format!("https://example.test/{tag}/greyscreen.exe"),
            size: 10,
            sha256: None,
        }),
    }
}

#[test]
fn previews_order_by_their_number() {
    assert!(v("v0.1.6-preview.8") < v("v0.1.10-preview.12"));
    assert!(v("0.1.6-preview.8") < v("0.1.6-preview.10"));
    assert!(v("0.1.6-preview.9") < v("0.1.6-preview.10"));
}

#[test]
fn a_stable_release_beats_its_previews() {
    assert!(v("0.2.0-preview.20") < v("0.2.0"));
    assert!(v("0.2.0") < v("0.2.1-preview.1"));
}

#[test]
fn versions_compare_numerically_not_as_text() {
    assert!(v("0.1.9") < v("0.1.10"));
    assert!(v("0.9.0") < v("0.10.0"));
}

#[test]
fn build_metadata_is_ignored() {
    assert_eq!(v("0.1.5+abcd1234-dirty"), v("0.1.5"));
}

#[test]
fn bad_tags_do_not_parse() {
    for bad in [
        "",
        "latest",
        "v1.2",
        "v1.2.3.4",
        "v1.2.x",
        "v1.2.3-",
        "v1.2.3-a..b",
        "v../x",
    ] {
        assert!(Version::parse(bad).is_none(), "{bad}");
    }
}

#[test]
fn a_preview_build_sees_previews_and_stable() {
    let releases = [
        release("v0.1.6-preview.8", true),
        release("v0.1.7-preview.9", true),
        release("v0.2.0", false),
    ];
    let running = v("0.1.6-preview.8");
    assert_eq!(
        newest_update(&running, &releases).map(|r| r.tag.as_str()),
        Some("v0.2.0")
    );
    let running = v("0.1.5-preview.7");
    assert_eq!(
        newest_update(&running, &releases[..2]).map(|r| r.tag.as_str()),
        Some("v0.1.7-preview.9")
    );
}

#[test]
fn a_stable_build_ignores_previews() {
    let releases = [release("v0.3.0-preview.30", true), release("v0.2.1", false)];
    let running = v("0.2.0");
    assert_eq!(
        newest_update(&running, &releases).map(|r| r.tag.as_str()),
        Some("v0.2.1")
    );
    assert!(newest_update(&v("0.2.1"), &releases).is_none());
}

#[test]
fn the_prerelease_flag_alone_marks_a_preview() {
    let releases = [release("v0.3.0", true)];
    assert!(newest_update(&v("0.2.0"), &releases).is_none());
}

#[test]
fn no_update_when_running_is_newest() {
    let releases = [release("v0.1.6-preview.8", true)];
    assert!(newest_update(&v("0.1.6-preview.8"), &releases).is_none());
    assert!(newest_update(&v("0.1.6"), &releases).is_none());
}

#[test]
fn releases_without_the_exe_or_with_odd_tags_are_skipped() {
    let mut no_exe = release("v0.9.0", false);
    no_exe.exe = None;
    let releases = [no_exe, release("nightly", false), release("v0.2.0", false)];
    assert_eq!(
        newest_update(&v("0.1.0"), &releases).map(|r| r.tag.as_str()),
        Some("v0.2.0")
    );
}

#[test]
fn digests_parse_only_when_valid_sha256() {
    let hex = "A".repeat(64);
    assert_eq!(parse_digest(&format!("sha256:{hex}")), Some("a".repeat(64)));
    assert_eq!(parse_digest("sha256:abc"), None);
    assert_eq!(parse_digest("sha1:abc"), None);
    assert_eq!(parse_digest(""), None);
}

#[test]
fn update_info_reads_the_release() {
    let info = UpdateInfo::from_release(&release("v0.1.6-preview.8", true));
    assert_eq!(info.version(), "0.1.6-preview.8");
    assert!(info.prerelease);
    assert_eq!(info.size, 10);
    assert_eq!(
        info.notes_url,
        "https://github.com/andresmarpz/greyscreen/releases/tag/v0.1.6-preview.8"
    );
}

#[test]
fn only_old_downloads_are_stale() {
    let running = v("0.1.6-preview.8");
    assert!(is_stale_download("greyscreen-v0.1.6-preview.8.exe", &running));
    assert!(is_stale_download("greyscreen-v0.1.5-preview.7.exe", &running));
    assert!(!is_stale_download(
        "greyscreen-v0.1.7-preview.9.exe",
        &running
    ));
    assert!(!is_stale_download("notes.txt", &running));
    assert!(!is_stale_download("greyscreen-latest.exe", &running));
}

#[test]
fn update_checks_are_on_by_default_and_for_old_settings_files() {
    use crate::settings::Settings;
    assert!(Settings::default().check_updates);
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert!(old.check_updates);
    let off: Settings = serde_json::from_str(r#"{"check_updates":false}"#).unwrap();
    assert!(!off.check_updates);
}
