#![cfg(windows)]

use std::time::Duration;

use greyscreen_capture::selftest;
use greyscreen_core::encoder::EncoderKind;
use greyscreen_core::settings::Settings;

const NVIDIA: u32 = 0x10de;
const RUNS: usize = 40;
const HEIGHT: u32 = 720;
const FPS: u32 = 30;

#[test]
#[ignore = "needs an NVIDIA GPU; 40 short encoder sessions in one process"]
fn a_hardware_encoder_released_right_after_start_does_not_crash() {
    let mut cfg = Settings::default().capture_config(HEIGHT);
    cfg.fps = FPS;
    let Some(probe) = selftest::probes()
        .into_iter()
        .find(|p| p.kind == EncoderKind::Hardware && p.vendor_id == NVIDIA)
    else {
        eprintln!("no NVIDIA hardware encoder, nothing to measure");
        return;
    };
    for run in 1..=RUNS {
        let summary = selftest::probe(&cfg, &probe).unwrap_or_else(|e| panic!("run {run}: {e}"));
        eprintln!("run {run}: {summary}");
        std::thread::sleep(Duration::from_millis(250));
    }
}
