#![cfg(windows)]
#![allow(clippy::expect_used)]

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use greyscreen_capture::audio::{AudioCapture, AudioCounters, AudioTarget};
use greyscreen_capture::clock::QpcMap;
use greyscreen_capture::{install, probe};
use greyscreen_core::health::CaptureStats;
use greyscreen_core::time::mono_now;
use wasapi::{DeviceEnumerator, Direction};
use windows::Win32::Foundation::E_ACCESSDENIED;
use windows::core::{Error, HRESULT};

const RATE: u64 = 48_000;
const SILENCE_WALL: Duration = Duration::from_secs(60);
const MAX_GAP: Duration = Duration::from_millis(100);
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
}

fn build_from_ver() -> u32 {
    let output = Command::new("cmd")
        .args(["/C", "ver"])
        .output()
        .expect("run ver");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.split('[')
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .and_then(|inner| inner.split_whitespace().last())
        .and_then(|version| version.split('.').nth(2))
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

fn has_render_endpoint() -> bool {
    wasapi::initialize_mta().ok().unwrap_or(());
    DeviceEnumerator::new()
        .and_then(|enumerator| enumerator.get_default_device(Direction::Render))
        .is_ok()
}

#[test]
fn loopback_audio_matches_wall_time_across_a_minute_of_silence() {
    if !has_render_endpoint() {
        eprintln!("no render endpoint, skipping");
        return;
    }
    let (tx, rx) = unbounded();
    let (t0_tx, t0_rx) = unbounded();
    let counters = Arc::new(AudioCounters::default());
    let capture = AudioCapture::start(
        AudioTarget::System,
        QpcMap::now(),
        t0_rx,
        tx,
        Arc::clone(&counters),
    )
    .expect("loopback start");
    t0_tx.send(mono_now()).expect("t0");
    let began = Instant::now();
    let mut last_arrival = began;
    let mut worst_gap = Duration::ZERO;
    let mut frames = 0u64;
    while let Some(left) = SILENCE_WALL.checked_sub(began.elapsed()) {
        match rx.recv_timeout(left.min(MAX_GAP * 4)) {
            Ok(block) => {
                let now = Instant::now();
                worst_gap = worst_gap.max(now - last_arrival);
                last_arrival = now;
                frames += (block.samples.len() / 2) as u64;
            }
            Err(_) => {
                worst_gap = worst_gap.max(last_arrival.elapsed());
            }
        }
    }
    let wall = began.elapsed().as_secs_f64();
    capture.stop();
    frames += rx
        .try_iter()
        .map(|block| (block.samples.len() / 2) as u64)
        .sum::<u64>();

    assert!(counters.take_failure().is_none());
    let mut stats = CaptureStats::default();
    counters.fill(&mut stats);
    assert!(
        stats.audio_gap_longest_ms <= MAX_GAP.as_millis() as u32,
        "{} ms",
        stats.audio_gap_longest_ms
    );
    assert!(worst_gap <= MAX_GAP, "gap of {worst_gap:?}");
    let audio = frames as f64 / RATE as f64;
    assert!(
        (audio - wall).abs() <= WALL_TOLERANCE,
        "audio {audio:.3}s against wall {wall:.3}s"
    );
}
