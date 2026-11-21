use super::*;

#[test]
fn a_copied_library_keeps_metadata_and_empties_videos() {
    let src = tempfile::tempdir().expect("src");
    let game = src.path().join("2026-11-21_13-00-00_Ahri_CLASSIC");
    fs::create_dir_all(game.join("clips")).expect("dirs");
    fs::write(game.join("game.json"), b"{}").expect("meta");
    fs::write(game.join("game.mp4"), vec![7u8; 4096]).expect("video");
    fs::write(game.join("clips/kill.MP4"), vec![7u8; 4096]).expect("clip");
    let out = tempfile::tempdir().expect("out");
    let sandbox = Sandbox {
        tmp: out.path().to_owned(),
    };
    let dest = prepare(out.path(), Some(src.path()), Path::new("unused"), &sandbox).expect("copy");
    let copy = dest.join("2026-11-21_13-00-00_Ahri_CLASSIC");
    assert_eq!(fs::read(copy.join("game.json")).expect("meta"), b"{}");
    assert_eq!(fs::metadata(copy.join("game.mp4")).expect("video").len(), 0);
    assert_eq!(
        fs::metadata(copy.join("clips/kill.MP4"))
            .expect("clip")
            .len(),
        0
    );
    assert_eq!(
        fs::metadata(game.join("game.mp4")).expect("source").len(),
        4096
    );
}

#[test]
fn the_output_cannot_live_inside_the_library() {
    let src = tempfile::tempdir().expect("src");
    let sandbox = Sandbox {
        tmp: src.path().to_owned(),
    };
    let err = prepare(
        &src.path().join("qa"),
        Some(src.path()),
        Path::new("unused"),
        &sandbox,
    );
    assert!(err.is_err());
}
