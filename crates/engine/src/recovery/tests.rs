use greyscreen_core::game::{GameInfo, GameResult, Player, Team};
use greyscreen_core::library::{GameRecord, META_VERSION, RecordingMeta, VideoInfo};

use super::*;

fn boxed(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(payload.len() + 8)
        .expect("size")
        .to_be_bytes()
        .to_vec();
    out.extend_from_slice(fourcc);
    out.extend_from_slice(payload);
    out
}

fn ftyp() -> Vec<u8> {
    boxed(b"ftyp", b"isom\0\0\x02\0isom")
}

fn small_mdat_header(payload_len: usize) -> Vec<u8> {
    let mut out = boxed(b"free", &[]);
    out.extend(boxed(b"mdat", &vec![7u8; payload_len]));
    out
}

fn large_mdat_header(payload_len: usize) -> Vec<u8> {
    let mut out = 1u32.to_be_bytes().to_vec();
    out.extend_from_slice(b"mdat");
    out.extend_from_slice(&u64::try_from(payload_len + 16).expect("size").to_be_bytes());
    out.extend(vec![7u8; payload_len]);
    out
}

fn mvhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
    let mut body = vec![0u8; 4];
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&timescale.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&[0; 80]);
    boxed(b"mvhd", &body)
}

fn mvhd_v1(timescale: u32, duration: u64) -> Vec<u8> {
    let mut body = vec![1u8, 0, 0, 0];
    body.extend_from_slice(&[0; 16]);
    body.extend_from_slice(&timescale.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&[0; 80]);
    boxed(b"mvhd", &body)
}

fn moov(mvhd: &[u8]) -> Vec<u8> {
    boxed(b"moov", mvhd)
}

fn file_of(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.concat()
}

fn meta(duration_secs: f64, video: &str) -> RecordingMeta {
    let me = Player {
        name: "Me".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    };
    RecordingMeta {
        version: META_VERSION,
        id: "game".into(),
        started_at: "2026-11-01T12:00:00[UTC]".parse().expect("zoned"),
        duration_secs,
        video: video.into(),
        game: GameRecord {
            info: GameInfo {
                mode: "CLASSIC".into(),
                map: Some(11),
                me: me.clone(),
                players: vec![me],
            },
            result: GameResult::Win,
        },
        events: Vec::new(),
        clips: Vec::new(),
        encoder: "test".into(),
        video_format: VideoInfo {
            width: 1920,
            height: 1080,
            fps: 60,
        },
        review: Default::default(),
        health: None,
    }
}

fn part_in(dir: &Path, bytes: &[u8]) -> PathBuf {
    let part = dir.join(PART_FILE);
    fs::write(&part, bytes).expect("part");
    part
}

#[test]
fn a_finalized_file_is_renamed_without_touching_its_bytes() {
    let finalized = [
        (
            "small header",
            file_of(&[ftyp(), small_mdat_header(40), moov(&mvhd_v0(1000, 5000))]),
        ),
        (
            "large header",
            file_of(&[ftyp(), large_mdat_header(40), moov(&mvhd_v0(1000, 5000))]),
        ),
        (
            "mdat right after ftyp",
            file_of(&[ftyp(), boxed(b"mdat", &[7u8; 40])]),
        ),
        (
            "wide ftyp",
            file_of(&[
                boxed(b"ftyp", &[b'x'; 40]),
                small_mdat_header(8),
                moov(&mvhd_v0(1000, 5000)),
            ]),
        ),
    ];
    for (name, bytes) in finalized {
        let dir = tempfile::tempdir().expect("dir");
        let part = part_in(dir.path(), &bytes);

        let finished = finish(&part).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert_eq!(finished, dir.path().join(VIDEO_FILE), "{name}");
        assert_eq!(fs::read(&finished).expect("video"), bytes, "{name}");
        assert!(!part.exists(), "{name}");
    }
}

#[test]
fn a_file_that_is_not_a_finished_recording_is_not_renamed_to_the_video() {
    let unfinished = [
        ("empty", Vec::new()),
        ("only part of the first box", ftyp()[..4].to_vec()),
        ("first box is not ftyp", boxed(b"junk", &[0u8; 40])),
        ("ftyp alone", ftyp()),
        (
            "ftyp then a short tail",
            file_of(&[ftyp(), boxed(b"free", &[])]),
        ),
        (
            "free box of the wrong size",
            file_of(&[ftyp(), boxed(b"free", &[0; 8]), boxed(b"mdat", &[7; 8])]),
        ),
        (
            "free box then something else",
            file_of(&[ftyp(), boxed(b"free", &[]), boxed(b"moof", &[7; 8])]),
        ),
        (
            "skip box where free belongs",
            file_of(&[ftyp(), boxed(b"skip", &[]), boxed(b"mdat", &[7; 8])]),
        ),
        (
            "large header that is not mdat",
            file_of(&[ftyp(), {
                let mut b = 1u32.to_be_bytes().to_vec();
                b.extend_from_slice(b"moof");
                b.extend_from_slice(&[0; 16]);
                b
            }]),
        ),
        (
            "mdat box of a different size field",
            file_of(&[ftyp(), {
                let mut b = 2u32.to_be_bytes().to_vec();
                b.extend_from_slice(b"mdat");
                b.extend_from_slice(&[0; 16]);
                b
            }]),
        ),
    ];
    for (name, bytes) in unfinished {
        let dir = tempfile::tempdir().expect("dir");
        let part = part_in(dir.path(), &bytes);

        let result = finish(&part);

        assert!(result.is_err(), "{name}: {result:?}");
        assert!(!dir.path().join(VIDEO_FILE).exists(), "{name}");
    }
}

#[test]
fn a_missing_part_is_an_error_not_a_recovery() {
    let dir = tempfile::tempdir().expect("dir");

    assert!(finish(&dir.path().join(PART_FILE)).is_err());
}

#[test]
fn version_zero_duration_is_the_duration_over_the_timescale() {
    assert_eq!(mvhd_duration(&moov(&mvhd_v0(1000, 90_500))), Some(90.5));
    assert_eq!(mvhd_duration(&moov(&mvhd_v0(600, 600))), Some(1.0));
}

#[test]
fn version_one_duration_reads_the_wide_field() {
    let wide = (1u64 << 32) + 1200;
    let expected = wide as f64 / 600.0;

    assert_eq!(mvhd_duration(&moov(&mvhd_v1(600, wide))), Some(expected));
}

#[test]
fn a_zero_timescale_has_no_duration() {
    assert_eq!(mvhd_duration(&moov(&mvhd_v0(0, 5000))), None);
    assert_eq!(mvhd_duration(&moov(&mvhd_v1(0, 5000))), None);
}

#[test]
fn a_moov_without_a_usable_mvhd_has_no_duration() {
    assert_eq!(mvhd_duration(&moov(&boxed(b"trak", &[0; 120]))), None);
    assert_eq!(mvhd_duration(&moov(&boxed(b"mvhd", &[0; 8]))), None);
    let mut cut = mvhd_v1(600, 6000);
    cut.truncate(30);
    assert_eq!(mvhd_duration(&moov(&cut)), None);
    assert_eq!(mvhd_duration(&mvhd_v0(1000, 5000)), None);
}

fn duration_of(bytes: &[u8]) -> Option<f64> {
    let dir = tempfile::tempdir().expect("dir");
    let video = dir.path().join(VIDEO_FILE);
    fs::write(&video, bytes).expect("video");
    movie_duration_secs(&video)
}

#[test]
fn the_movie_duration_is_found_after_any_kind_of_box_header() {
    let moov = moov(&mvhd_v0(1000, 12_000));
    assert_eq!(
        duration_of(&file_of(&[ftyp(), small_mdat_header(100), moov.clone()])),
        Some(12.0)
    );
    assert_eq!(
        duration_of(&file_of(&[ftyp(), large_mdat_header(100), moov.clone()])),
        Some(12.0)
    );
    assert_eq!(duration_of(&moov), Some(12.0));
}

#[test]
fn a_moov_whose_size_is_zero_runs_to_the_end_of_the_file() {
    let mut moov = moov(&mvhd_v0(1000, 7_000));
    moov[..4].copy_from_slice(&0u32.to_be_bytes());

    assert_eq!(
        duration_of(&file_of(&[ftyp(), small_mdat_header(100), moov])),
        Some(7.0)
    );
}

#[test]
fn a_file_without_a_readable_moov_has_no_duration() {
    assert_eq!(duration_of(&[]), None);
    assert_eq!(
        duration_of(&file_of(&[ftyp(), small_mdat_header(100)])),
        None
    );
    assert_eq!(
        duration_of(&file_of(&[ftyp(), boxed(b"free", &[0; 2])])),
        None
    );
    let mut broken = file_of(&[ftyp()]);
    broken.extend_from_slice(&4u32.to_be_bytes());
    broken.extend_from_slice(b"free");
    broken.extend(moov(&mvhd_v0(1000, 1000)));
    assert_eq!(duration_of(&broken), None);
    let mut cut = file_of(&[ftyp(), moov(&mvhd_v0(1000, 1000))]);
    cut.truncate(cut.len() - 20);
    assert_eq!(duration_of(&cut), None);
}

fn sidecar_after_recovery(meta: RecordingMeta) -> RecordingMeta {
    let dir = tempfile::tempdir().expect("dir");
    library::write_meta(dir.path(), &meta).expect("sidecar");
    let bytes = file_of(&[ftyp(), small_mdat_header(40), moov(&mvhd_v0(1000, 90_500))]);
    let part = part_in(dir.path(), &bytes);

    recover_part(&part);

    assert!(dir.path().join(VIDEO_FILE).is_file());
    library::read_meta(dir.path()).expect("sidecar")
}

#[test]
fn recovery_fills_in_a_missing_duration_and_video_from_the_file() {
    let recovered = sidecar_after_recovery(meta(0.0, ""));

    assert_eq!(recovered.duration_secs, 90.5);
    assert_eq!(recovered.video, VIDEO_FILE);
}

#[test]
fn recovery_keeps_a_duration_and_video_the_sidecar_already_has() {
    let recovered = sidecar_after_recovery(meta(12.0, "other.mp4"));

    assert_eq!(recovered.duration_secs, 12.0);
    assert_eq!(recovered.video, "other.mp4");
}

#[test]
fn recovery_replaces_a_negative_duration() {
    assert_eq!(
        sidecar_after_recovery(meta(-1.0, VIDEO_FILE)).duration_secs,
        90.5
    );
}

#[test]
fn recovery_without_a_sidecar_still_finishes_the_video() {
    let dir = tempfile::tempdir().expect("dir");
    let part = part_in(dir.path(), &file_of(&[ftyp(), small_mdat_header(8)]));

    recover_part(&part);

    assert!(dir.path().join(VIDEO_FILE).is_file());
    assert!(!dir.path().join("game.json").exists());
}

#[test]
fn parts_are_found_one_folder_down() {
    let dir = tempfile::tempdir().expect("dir");
    let with = dir.path().join("a");
    let without = dir.path().join("b");
    fs::create_dir_all(&with).expect("dir");
    fs::create_dir_all(&without).expect("dir");
    part_in(&with, b"x");
    fs::write(without.join(VIDEO_FILE), b"x").expect("video");
    fs::write(dir.path().join(PART_FILE), b"x").expect("stray");

    assert_eq!(find_parts(dir.path()), vec![with.join(PART_FILE)]);
    assert!(find_parts(&dir.path().join("missing")).is_empty());
}
