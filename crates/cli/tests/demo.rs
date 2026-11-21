#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use greyscreen_core::library;

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_greyscreen-cli"))
        .args(args)
        .output()
        .expect("run cli")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("greyscreen-cli-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn ffprobe_ok(path: &Path) {
    let Ok(out) = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration"])
        .arg(path)
        .output()
    else {
        return;
    };
    assert!(
        out.status.success(),
        "ffprobe rejected {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn seed_writes_readable_games_and_clear_keeps_other_folders() {
    let dir = scratch("demo");
    let out = cli(&[
        "demo",
        "seed",
        "--dir",
        dir.to_str().unwrap(),
        "--games",
        "2",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let metas: Vec<_> = library::scan(&dir)
        .into_iter()
        .filter(|m| m.encoder == "demo")
        .collect();
    assert_eq!(metas.len(), 2);
    for meta in &metas {
        assert!(!meta.events.is_empty());
        assert!(!meta.clips.is_empty());
        let folder = dir.join(&meta.id);
        ffprobe_ok(&folder.join(&meta.video));
        for clip in &meta.clips {
            ffprobe_ok(&folder.join(&clip.file));
        }
    }

    let mut real = metas[0].clone();
    real.encoder = "nvenc".into();
    library::write_meta(&dir.join(&real.id), &real).unwrap();

    let out = cli(&["demo", "clear", "--dir", dir.to_str().unwrap()]);
    assert!(out.status.success());
    let left = library::scan(&dir);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, real.id);
    std::fs::remove_dir_all(&dir).ok();
}
