#![cfg(windows)]
#![allow(clippy::expect_used)]

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use greyscreen_capture::audio::{AudioCapture, AudioCounters, AudioTarget};
use greyscreen_capture::clock::QpcMap;
use greyscreen_capture::{install, probe};
use greyscreen_core::time::mono_now;
use windows::Win32::Foundation::E_ACCESSDENIED;
use windows::core::{Error, HRESULT};

const RATE: u64 = 48_000;
const SILENCE_WALL: Duration = Duration::from_secs(60);
const MAX_GAP_FRAMES: u64 = RATE / 10;
const WALL_TOLERANCE: f64 = 0.25;

#[test]
fn a_windows_error_formats_with_its_hex_code_and_keeps_non_ascii_text() {
    let text = "ファイルを開けません é ü";
    let error = Error::new(HRESULT(0x8007_0005_u32 as i32), text);
    let shown = error.to_string();
    assert!(shown.contains(text), "{shown}");
    assert!(shown.to_uppercase().contains("0X80070005"), "{shown}");

    let bare = Error::from_hresult(E_ACCESSDENIED).to_string();
    assert!(bare.to_uppercase().contains("0X80070005"), "{bare}");
}

#[test]
fn install_paths_are_absolute() {
    let dir = install::install_dir().expect("install dir");
    assert!(dir.is_absolute(), "{}", dir.display());
    let exe = std::env::current_exe().expect("current exe");
    assert!(exe.is_absolute(), "{}", exe.display());
}

fn build_from_ver() -> u32 {
    let output = Command::new("cmd")
        .args(["/C", "ver"])
        .output()
        .expect("run ver");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let version = text
        .split("Version")
        .nth(1)
        .expect("ver prints a version")
        .trim()
        .trim_end_matches(']')
        .trim()
        .to_owned();
    version
        .split('.')
        .nth(2)
        .and_then(|build| build.parse().ok())
        .expect("ver build number")
}

#[test]
fn the_windows_build_is_ten_above_17763_and_matches_ver() {
    let build = probe::windows_build();
    let parts: Vec<u32> = build
        .split('.')
        .map(|part| part.parse().expect("numeric version part"))
        .collect();
    assert_eq!(parts.len(), 3, "{build}");
    assert_eq!(parts[0], 10, "{build}");
    assert!(parts[2] > 17763, "{build}");
    assert_eq!(parts[2], build_from_ver());
}

#[test]
fn loopback_audio_matches_wall_time_across_a_minute_of_silence() {
    let (tx, rx) = unbounded();
    let (t0_tx, t0_rx) = unbounded();
    let counters = Arc::new(AudioCounters::default());
    let capture = match AudioCapture::start(
        AudioTarget::System,
        QpcMap::now(),
        t0_rx,
        tx,
        Arc::clone(&counters),
    ) {
        Ok(capture) => capture,
        Err(error) => {
            eprintln!("no loopback device, skipping: {error}");
            return;
        }
    };
    t0_tx.send(mono_now()).expect("t0");
    let began = Instant::now();
    std::thread::sleep(SILENCE_WALL);
    let wall = began.elapsed().as_secs_f64();
    capture.stop();

    let mut next = 0u64;
    let mut worst_gap = 0u64;
    for block in rx.try_iter() {
        worst_gap = worst_gap.max(block.first_frame.saturating_sub(next));
        next = next.max(block.first_frame + (block.samples.len() / 2) as u64);
    }
    let audio = next as f64 / RATE as f64;
    assert!(worst_gap <= MAX_GAP_FRAMES, "gap of {worst_gap} frames");
    assert!(
        (audio - wall).abs() <= WALL_TOLERANCE,
        "audio {audio:.3}s against wall {wall:.3}s"
    );
}
