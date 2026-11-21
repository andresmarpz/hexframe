#![allow(clippy::expect_used)]

#[cfg(not(windows))]
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_greyscreen-cli"))
        .args(args)
        .output()
        .expect("run cli")
}

#[cfg(not(windows))]
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("greyscreen-cli-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

#[cfg(not(windows))]
fn ffprobe_duration(path: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    assert!(
        out.status.success(),
        "ffprobe rejected the file: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

#[cfg(not(windows))]
#[test]
fn record_writes_a_playable_file_with_the_asked_duration() {
    let dir = scratch("record");
    let out = dir.join("a.mp4");
    let result = cli(&[
        "record",
        "--seconds",
        "2",
        "--out",
        out.to_str().expect("utf8"),
    ]);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success(), "{stdout}");
    assert!(stdout.contains("encoder: fake"), "{stdout}");
    assert!(out.exists());
    assert!(!dir.join("a.mp4.part").exists());
    if let Some(duration) = ffprobe_duration(&out) {
        assert!((duration - 2.0).abs() <= 0.2, "duration {duration}");
    }
}

#[cfg(not(windows))]
#[test]
fn record_rejects_window_search_off_windows() {
    let result = cli(&["record", "--seconds", "1", "--window", "x"]);
    assert!(!result.status.success());
}

#[test]
fn record_rejects_a_zero_duration() {
    let result = cli(&["record", "--seconds", "0"]);
    assert!(!result.status.success());
}

#[cfg(not(windows))]
#[test]
fn recover_rebuilds_an_abandoned_part_file() {
    use greyscreen_core::media::{CaptureTarget, MediaEvent, MediaSource, Track};
    use greyscreen_core::settings::Settings;
    use greyscreen_engine::FakeMediaSource;
    use greyscreen_mp4::fragmented::FragmentedWriter;

    let dir = scratch("recover");
    let part = dir.join("b.mp4.part");
    let (tx, rx) = crossbeam_channel::bounded(4096);
    let session = FakeMediaSource::fast()
        .start(
            CaptureTarget::PrimaryMonitor,
            Settings::default().capture_config(2160),
            tx,
        )
        .expect("start");
    let mut writer = None;
    while let Ok(event) = rx.recv() {
        match event {
            MediaEvent::Started { video, audio, .. } => {
                writer =
                    Some(FragmentedWriter::create(&part, &video, audio.as_ref()).expect("create"));
            }
            MediaEvent::Packet(packet) => {
                writer
                    .as_mut()
                    .expect("started")
                    .push(&packet)
                    .expect("push");
                if packet.dts > 3 * 90_000 && packet.track == Track::Video {
                    break;
                }
            }
            _ => {}
        }
    }
    drop(writer);
    drop(rx);
    session.stop();

    let result = cli(&["recover", part.to_str().expect("utf8")]);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success(), "{stdout}");
    let recovered = dir.join("b.mp4");
    assert!(recovered.exists());
    assert!(!part.exists());
    if let Some(duration) = ffprobe_duration(&recovered) {
        assert!(duration > 1.0, "duration {duration}");
    }
}
