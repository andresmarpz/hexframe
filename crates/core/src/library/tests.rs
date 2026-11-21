use jiff::civil::date;
use jiff::tz::TimeZone;

use super::*;
use crate::game::{Player, Team};

fn zoned(day: i8, hour: i8) -> Zoned {
    date(2026, 11, day)
        .at(hour, 55, 1, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
}

fn player() -> Player {
    Player {
        name: "Me".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    }
}

fn meta(id: &str, started_at: Zoned) -> RecordingMeta {
    RecordingMeta {
        version: META_VERSION,
        id: id.into(),
        started_at,
        duration_secs: 1800.5,
        video: "game.mp4".into(),
        game: GameRecord {
            info: GameInfo {
                mode: "CLASSIC".into(),
                map: Some(11),
                me: player(),
                players: vec![player()],
            },
            result: GameResult::Win,
        },
        events: vec![EventEntry {
            id: 3,
            game_secs: 100.0,
            rec_secs: 112.5,
            kind: GameEvent::FirstBlood {
                recipient: "Me".into(),
            },
            highlights: vec!["Kill".into()],
        }],
        clips: vec![ClipEntry {
            file: "clips/01-52_Kill.mp4".into(),
            start_secs: 97.5,
            end_secs: 117.5,
            label: "Kill".into(),
        }],
        encoder: "NVIDIA H.264".into(),
        video_format: VideoInfo {
            width: 1920,
            height: 1080,
            fps: 60,
        },
        review: Default::default(),
        health: None,
    }
}

#[test]
fn folder_name_formats_and_sanitises() {
    assert_eq!(
        folder_name(zoned(20, 19), "Ahri", "CLASSIC"),
        "2026-11-20_19-55-01_Ahri_CLASSIC"
    );
    assert_eq!(
        folder_name(zoned(20, 19), "Kai'Sa", "ARAM/Mayhem"),
        "2026-11-20_19-55-01_KaiSa_ARAMMayhem"
    );
    assert_eq!(
        folder_name(zoned(20, 19), "..", "CLASSIC"),
        "2026-11-20_19-55-01_Unknown_CLASSIC"
    );
}

#[test]
fn clip_file_name_uses_minutes_and_seconds() {
    assert_eq!(clip_file_name(754.9, "MyKill"), "12-34_MyKill.mp4");
    assert_eq!(clip_file_name(5.0, "Kill+Death"), "00-05_Kill-Death.mp4");
    assert_eq!(clip_file_name(6000.0, "A/B"), "100-00_AB.mp4");
    assert_eq!(clip_file_name(-3.0, "!!"), "00-00_Clip.mp4");
}

#[test]
fn meta_round_trips_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let original = meta("a", zoned(20, 19));
    write_meta(dir.path(), &original).unwrap();
    assert_eq!(read_meta(dir.path()).unwrap(), original);
    assert!(!dir.path().join("game.json.tmp").exists());
}

#[test]
fn sidecar_json_has_the_documented_shape() {
    let json = serde_json::to_value(meta("a", zoned(20, 19))).unwrap();
    assert_eq!(json["version"], 1);
    assert_eq!(json["game"]["result"], "win");
    assert_eq!(json["game"]["mode"], "CLASSIC");
    assert_eq!(json["game"]["me"]["champion"], "Ahri");
    assert_eq!(json["events"][0]["kind"]["type"], "first_blood");
    assert_eq!(json["video_format"]["fps"], 60);
    assert!(
        json["started_at"]
            .as_str()
            .unwrap()
            .starts_with("2026-11-20T19:55:01")
    );
}

#[test]
fn write_meta_replaces_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut m = meta("a", zoned(20, 19));
    write_meta(dir.path(), &m).unwrap();
    m.clips.clear();
    write_meta(dir.path(), &m).unwrap();
    assert!(read_meta(dir.path()).unwrap().clips.is_empty());
}

#[test]
fn read_meta_rejects_unknown_versions() {
    let dir = tempfile::tempdir().unwrap();
    let mut m = meta("a", zoned(20, 19));
    m.version = 2;
    write_meta(dir.path(), &m).unwrap();
    assert!(matches!(
        read_meta(dir.path()),
        Err(LibraryError::Version(2))
    ));
}

#[test]
fn scan_sorts_newest_first_and_skips_bad_entries() {
    let root = tempfile::tempdir().unwrap();
    for (name, m) in [
        ("old", meta("old", zoned(18, 10))),
        ("new", meta("new", zoned(20, 19))),
        ("mid", meta("mid", zoned(19, 12))),
    ] {
        write_meta(&root.path().join(name), &m).unwrap();
    }
    let broken = root.path().join("broken");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join(META_FILE), "{ not json").unwrap();
    fs::create_dir_all(root.path().join("empty")).unwrap();
    fs::write(root.path().join("stray.txt"), "x").unwrap();

    let ids: Vec<_> = scan(root.path()).into_iter().map(|m| m.id).collect();
    assert_eq!(ids, ["new", "mid", "old"]);
}

#[test]
fn scan_of_a_missing_directory_is_empty() {
    let root = tempfile::tempdir().unwrap();
    assert!(scan(&root.path().join("nope")).is_empty());
}

#[test]
fn an_unreviewed_sidecar_has_no_review_section() {
    let json = serde_json::to_value(meta("a", zoned(20, 19))).unwrap();
    assert!(json.get("review").is_none());
}

#[test]
fn a_sidecar_from_before_reviews_still_reads() {
    let mut json = serde_json::to_value(meta("a", zoned(20, 19))).unwrap();
    json.as_object_mut().unwrap().remove("review");
    let read: RecordingMeta = serde_json::from_value(json).unwrap();
    assert!(read.review.is_empty());
}

#[test]
fn tags_and_notes_persist_in_the_sidecar() {
    use crate::review::DeathTag;

    let dir = tempfile::tempdir().unwrap();
    let original = meta("a", zoned(20, 19));
    write_meta(dir.path(), &original).unwrap();

    let mut review = ReviewData::default();
    review.toggle_tag(3, DeathTag::NoVision);
    review.toggle_tag(3, DeathTag::Ganked);
    review.set_note(3, "No ward in river");
    write_review(dir.path(), &review).unwrap();

    let read = read_meta(dir.path()).unwrap();
    assert_eq!(read.review, review);
    assert_eq!(read.events, original.events);
    assert_eq!(read.clips, original.clips);

    let raw: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(META_FILE)).unwrap()).unwrap();
    assert_eq!(raw["review"]["deaths"][0]["event_id"], 3);
    assert_eq!(raw["review"]["deaths"][0]["tags"][0], "no_vision");
    assert_eq!(raw["review"]["deaths"][0]["note"], "No ward in river");
}

#[test]
fn clearing_every_review_removes_the_section_again() {
    let dir = tempfile::tempdir().unwrap();
    write_meta(dir.path(), &meta("a", zoned(20, 19))).unwrap();
    let mut review = ReviewData::default();
    review.toggle_tag(3, crate::review::DeathTag::Ganked);
    write_review(dir.path(), &review).unwrap();
    review.toggle_tag(3, crate::review::DeathTag::Ganked);
    write_review(dir.path(), &review).unwrap();
    let raw: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(META_FILE)).unwrap()).unwrap();
    assert!(raw.get("review").is_none());
}

#[test]
fn write_review_fails_without_a_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    assert!(write_review(dir.path(), &ReviewData::default()).is_err());
    assert!(!dir.path().join(META_FILE).exists());
}

#[test]
fn clip_thumbnails_sit_next_to_their_clip() {
    let clip = &meta("a", zoned(20, 19)).clips[0];
    assert_eq!(thumb::clip_thumb_file(clip), "clips/01-52_Kill.jpg");
}

#[test]
fn clip_thumbnail_time_lands_near_the_moment_and_inside_the_clip() {
    let mut clip = meta("a", zoned(20, 19)).clips.remove(0);
    assert!((thumb::clip_thumb_secs(&clip) - 14.0).abs() < 1e-9);
    clip.end_secs = clip.start_secs + 0.5;
    assert_eq!(thumb::clip_thumb_secs(&clip), 0.0);
}

#[test]
fn game_thumbnail_uses_the_first_clip_then_falls_back_to_the_game() {
    let mut m = meta("a", zoned(20, 19));
    assert!((thumb::game_thumb_secs(&m) - 111.5).abs() < 1e-9);
    m.clips.clear();
    assert!((thumb::game_thumb_secs(&m) - 720.2).abs() < 1e-9);
    m.duration_secs = 0.5;
    assert_eq!(thumb::game_thumb_secs(&m), 0.0);
}

#[test]
fn a_filter_override_persists_in_the_sidecar_next_to_the_death_notes() {
    use crate::categories::{CategorySet, EventCategory};
    use crate::review::DeathTag;

    let dir = tempfile::tempdir().unwrap();
    write_meta(dir.path(), &meta("a", zoned(20, 19))).unwrap();

    let mut review = ReviewData::default();
    review.toggle_tag(3, DeathTag::Ganked);
    let filter = CategorySet::NONE
        .with(EventCategory::MyDeaths, true)
        .with(EventCategory::Barons, true);
    review.filter = Some(filter);
    write_review(dir.path(), &review).unwrap();

    let read = read_meta(dir.path()).unwrap();
    assert_eq!(read.review.filter, Some(filter));
    assert_eq!(read.review.deaths, review.deaths);
    let raw: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(META_FILE)).unwrap()).unwrap();
    assert_eq!(
        raw["review"]["filter"],
        serde_json::json!(["my_deaths", "barons"])
    );
}

#[test]
fn a_filter_alone_keeps_the_review_section_and_clearing_it_removes_it() {
    let dir = tempfile::tempdir().unwrap();
    write_meta(dir.path(), &meta("a", zoned(20, 19))).unwrap();
    let mut review = ReviewData {
        filter: Some(crate::categories::CategorySet::all()),
        ..ReviewData::default()
    };
    write_review(dir.path(), &review).unwrap();
    assert!(read_meta(dir.path()).unwrap().review.filter.is_some());
    review.filter = None;
    write_review(dir.path(), &review).unwrap();
    let raw: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(META_FILE)).unwrap()).unwrap();
    assert!(raw.get("review").is_none());
}

#[test]
fn a_sidecar_without_a_filter_reads_as_no_override() {
    let json = serde_json::json!({"deaths": [{"event_id": 3, "tags": [], "note": "x"}]});
    let review: ReviewData = serde_json::from_value(json).unwrap();
    assert_eq!(review.filter, None);
}

#[test]
fn a_sidecar_without_health_has_no_health_section_and_old_ones_still_read() {
    let mut json = serde_json::to_value(meta("a", zoned(20, 19))).unwrap();
    assert!(json.get("health").is_none());
    json.as_object_mut().unwrap().remove("health");
    let read: RecordingMeta = serde_json::from_value(json).unwrap();
    assert!(read.health.is_none());
}

#[test]
fn health_survives_a_write_and_read_of_the_sidecar() {
    let root = tempfile::tempdir().unwrap();
    let mut with_health = meta("a", zoned(20, 19));
    with_health.health = Some(RecordingHealth {
        target_fps: 60,
        frames_encoded: 600,
        dropped_pct: 0.5,
        ..RecordingHealth::default()
    });
    write_meta(root.path(), &with_health).unwrap();
    let read = read_meta(root.path()).unwrap();
    assert_eq!(read.health, with_health.health);
    let raw: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join(META_FILE)).unwrap()).unwrap();
    assert_eq!(raw["health"]["frames_encoded"], 600);
}
