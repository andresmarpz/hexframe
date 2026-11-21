use std::path::Path;

use super::scrub::tail_lines;
use super::*;

fn draft() -> FeedbackDraft {
    FeedbackDraft {
        kind: FeedbackKind::Bug,
        message: "  The clip button does nothing.  ".into(),
        email: " me@example.com ".into(),
        include_logs: true,
        client_id: "client-1".into(),
    }
}

fn context(home_dirs: &[String]) -> ReportContext<'_> {
    ReportContext {
        app_version: "0.1.9",
        os: "Windows 10.0.22631",
        home_dirs,
    }
}

#[test]
fn the_report_carries_the_contract_fields() {
    let report = build_report(&draft(), context(&[]), || Some("one\n".into()));
    let json: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(json["kind"], "bug");
    assert_eq!(json["message"], "The clip button does nothing.");
    assert_eq!(json["email"], "me@example.com");
    assert_eq!(json["app_version"], "0.1.9");
    assert_eq!(json["os"], "Windows 10.0.22631");
    assert_eq!(json["log_tail"], "one\n");
    assert_eq!(json["client_id"], "client-1");
}

#[test]
fn kinds_serialize_in_lower_case() {
    let kinds: Vec<_> = FeedbackKind::ALL
        .iter()
        .map(|kind| serde_json::to_string(kind).unwrap())
        .collect();
    assert_eq!(kinds, ["\"bug\"", "\"idea\"", "\"other\""]);
}

#[test]
fn an_empty_email_is_null() {
    let mut blank = draft();
    blank.email = "   ".into();
    let report = build_report(&blank, context(&[]), || None);
    assert_eq!(report.email, None);
    assert!(report.to_json().contains("\"email\": null"));
}

#[test]
fn logs_are_not_read_or_sent_when_the_box_is_not_ticked() {
    let mut off = draft();
    off.include_logs = false;
    let report = build_report(&off, context(&[]), || {
        panic!("the log must not be read");
    });
    assert_eq!(report.log_tail, None);
    assert!(report.to_json().contains("\"log_tail\": null"));
}

#[test]
fn a_missing_log_file_still_sends_the_report() {
    let report = build_report(&draft(), context(&[]), || None);
    assert_eq!(report.log_tail, None);
}

#[test]
fn only_the_last_300_lines_are_kept() {
    let raw: String = (1..=450).map(|n| format!("line {n}\n")).collect();
    let tail = scrub_logs(&raw, &[]);
    assert_eq!(tail.lines().count(), 300);
    assert!(tail.starts_with("line 151\n"));
    assert!(tail.ends_with("line 450\n"));
}

#[test]
fn a_short_log_is_kept_whole() {
    assert_eq!(tail_lines("a\nb\n", 300), "a\nb\n");
    assert_eq!(tail_lines("a\nb", 300), "a\nb");
    assert_eq!(tail_lines("a\nb\nc\n", 2), "b\nc\n");
    assert_eq!(tail_lines("", 300), "");
}

#[test]
fn the_home_folder_becomes_a_tilde_in_every_spelling() {
    let homes = home_variants(Path::new("C:\\Users\\Ana"));
    let raw = concat!(
        "open C:\\Users\\Ana\\Videos\\Greyscreen\n",
        "path=\"C:\\\\Users\\\\Ana\\\\AppData\"\n",
        "also c:/users/ana/Desktop and C:\\USERS\\ANA\\x\n"
    );
    let scrubbed = scrub_logs(raw, &homes);
    assert!(!scrubbed.to_lowercase().contains("ana"), "{scrubbed}");
    assert!(scrubbed.contains("open ~\\Videos\\Greyscreen"));
    assert!(scrubbed.contains("path=\"~\\\\AppData\""));
    assert!(scrubbed.contains("also ~/Desktop and ~\\x"));
}

#[test]
fn a_unix_home_is_replaced_too() {
    let homes = home_variants(Path::new("/Users/ana"));
    assert_eq!(
        scrub_logs("saved /Users/ana/Movies/a.mp4", &homes),
        "saved ~/Movies/a.mp4"
    );
}

#[test]
fn riot_names_are_redacted_in_log_fields_and_json() {
    let raw = concat!(
        "INFO game ready summoner=Faker mode=CLASSIC\n",
        "INFO player riotId=\"Hide on bush#KR1\" champion=Ahri\n",
        "DEBUG {\"summonerName\":\"Ana Maria\",\"riotIdGameName\": \"Ana\",\"riotIdTagLine\":\"EUW\"}\n",
        "INFO riotIdGameName=Ana tagline=EUW\n",
    );
    let scrubbed = scrub_logs(raw, &[]);
    for name in ["Faker", "Hide on bush", "KR1", "Ana", "EUW"] {
        assert!(!scrubbed.contains(name), "{name} leaked in {scrubbed}");
    }
    assert!(scrubbed.contains("mode=CLASSIC"));
    assert!(scrubbed.contains("champion=Ahri"));
    assert!(scrubbed.contains("summoner=<redacted> mode"));
}

#[test]
fn similar_words_are_left_alone() {
    let raw = "summoners=3 mysummoner=x tagline_count=4 summoner rift\n";
    assert_eq!(scrub_logs(raw, &[]), raw);
}

#[test]
fn non_ascii_text_survives_scrubbing() {
    let raw = "partie terminée summoner=\"Jürgen Ünal\" ok → fini\n";
    assert_eq!(
        scrub_logs(raw, &[]),
        "partie terminée summoner=\"<redacted>\" ok → fini\n"
    );
}

#[test]
fn the_draft_needs_a_message_within_the_limit() {
    let mut empty = draft();
    empty.message = " \n ".into();
    assert_eq!(empty.problem(), Some(DraftProblem::EmptyMessage));
    let mut long = draft();
    long.message = "a".repeat(MAX_MESSAGE_CHARS + 1);
    assert_eq!(long.problem(), Some(DraftProblem::MessageTooLong));
    long.message = "é".repeat(MAX_MESSAGE_CHARS);
    assert_eq!(long.problem(), None);
}

#[test]
fn the_email_is_optional_but_must_look_like_one() {
    let mut mail = draft();
    mail.email = String::new();
    assert_eq!(mail.problem(), None);
    for bad in ["me", "me@", "@x.com", "me@x", "me @x.com", "me@x."] {
        mail.email = bad.into();
        assert_eq!(mail.problem(), Some(DraftProblem::BadEmail), "{bad}");
    }
    mail.email = "me@x.io".into();
    assert_eq!(mail.problem(), None);
}

#[test]
fn the_newest_log_file_is_read() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("greyscreen.log.2026-11-20"), "old\n").unwrap();
    std::fs::write(dir.path().join("greyscreen.log.2026-11-21"), "new\n").unwrap();
    std::fs::write(dir.path().join("crash.txt"), "other\n").unwrap();
    assert_eq!(read_newest_log(dir.path()).as_deref(), Some("new\n"));
    assert_eq!(read_newest_log(&dir.path().join("missing")), None);
}

#[test]
fn a_huge_log_is_read_from_its_end() {
    let dir = tempfile::tempdir().unwrap();
    let big = format!("{}tail-marker\n", "x".repeat(3 * 1024 * 1024));
    std::fs::write(dir.path().join("greyscreen.log.2026-11-21"), big).unwrap();
    let read = read_newest_log(dir.path()).unwrap();
    assert!(read.len() <= 1024 * 1024 + 16);
    assert!(read.ends_with("tail-marker\n"));
}

#[test]
fn client_ids_are_random_version_4_uuids() {
    let a = new_client_id();
    let b = new_client_id();
    assert_ne!(a, b);
    assert_eq!(a.len(), 36);
    let parts: Vec<_> = a.split('-').collect();
    assert_eq!(
        parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
        [8, 4, 4, 4, 12]
    );
    assert!(parts[2].starts_with('4'));
    assert!(matches!(parts[3].as_bytes()[0], b'8' | b'9' | b'a' | b'b'));
    assert!(a.bytes().all(|c| c == b'-' || c.is_ascii_hexdigit()));
}
