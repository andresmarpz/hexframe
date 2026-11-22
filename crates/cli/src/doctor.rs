#[cfg_attr(not(windows), allow(dead_code))]
mod deep;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::process::ExitCode;

use greyscreen_core::paths;
#[cfg(windows)]
use greyscreen_core::settings::NATIVE_HEIGHT_CEILING;
use greyscreen_core::settings::Settings;

pub(crate) fn run(deep: bool, out: &Path) -> ExitCode {
    print_environment();
    let basic = capture_checks();
    if !deep {
        return basic;
    }
    if deep_checks(out) {
        basic
    } else {
        ExitCode::FAILURE
    }
}

fn print_environment() {
    let settings_path = paths::settings_file();
    let settings = Settings::load(&settings_path);
    println!("greyscreen-cli {}", env!("CARGO_PKG_VERSION"));
    println!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!("settings: {}", settings_path.display());
    println!("logs: {}", paths::logs_dir().display());
    println!("output: {}", settings.output_dir.display());
}

#[cfg(windows)]
fn capture_checks() -> ExitCode {
    use greyscreen_capture::probe;

    crate::init_windows();
    print!("{}", probe::system_report());
    let encoder_ok = encoder_checks();
    let capture_ok = match capture_test() {
        Ok((frames, width, height, encoder)) => {
            println!("capture: OK, {frames} frames, {width}x{height}, {encoder}");
            true
        }
        Err(reason) => {
            println!("capture: FAILED {reason}");
            false
        }
    };
    if encoder_ok && capture_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(windows)]
fn encoder_checks() -> bool {
    use std::sync::Arc;
    use std::time::Duration;

    use greyscreen_capture::WindowsMediaSource;
    use greyscreen_core::encoder::{CheckOutcome, EncoderCheck};
    use greyscreen_engine::{ChildRunner, ProbeRunner};

    const PROBE_DEADLINE: Duration = Duration::from_secs(10);

    let settings = Settings::load(&paths::settings_file());
    let cfg = settings.capture_config(NATIVE_HEIGHT_CEILING);
    let Ok(exe) = std::env::current_exe() else {
        println!("encoder check: FAILED cannot find greyscreen-cli to run the check in a child");
        return false;
    };
    let runner = ChildRunner::new(Arc::new(WindowsMediaSource::new()), exe, PROBE_DEADLINE);
    let attempts: Vec<_> = runner
        .probes()
        .iter()
        .map(|probe| runner.run(&cfg, probe).into_attempt(probe))
        .collect();
    println!("encoder check (every encoder, your settings):");
    for (index, attempt) in attempts.iter().enumerate() {
        let result = match &attempt.result {
            Ok(detail) => format!("OK, {detail}"),
            Err(error) => format!("FAILED {error}"),
        };
        println!(
            "  {}. {} on {}: {result}",
            index + 1,
            attempt.label(),
            attempt.adapter
        );
    }
    let check = EncoderCheck::from_attempts(String::new(), &attempts);
    match check.outcome {
        CheckOutcome::Hardware => println!("encoder: {}", check.encoder),
        CheckOutcome::Software => {
            println!("encoder: {} (no hardware encoder works)", check.encoder)
        }
        CheckOutcome::Failed => println!("encoder: FAILED, none works"),
    }
    !check.failed()
}

#[cfg(windows)]
fn capture_test() -> Result<(u64, u32, u32, String), String> {
    use std::time::{Duration, Instant};

    use greyscreen_capture::WindowsMediaSource;
    use greyscreen_core::media::{CaptureTarget, MediaEvent, MediaSource, Track};

    const TEST_DURATION: Duration = Duration::from_secs(2);

    let mut cfg = Settings::default().capture_config(NATIVE_HEIGHT_CEILING);
    cfg.fps = 30;
    cfg.audio = false;
    let (tx, rx) = crossbeam_channel::bounded(1024);
    let session = WindowsMediaSource::new().start(CaptureTarget::PrimaryMonitor, cfg, tx)?;
    let deadline = Instant::now() + TEST_DURATION;
    let mut size = None;
    let mut encoder = String::new();
    let mut frames = 0u64;
    let mut failure = None;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(MediaEvent::Started {
                video, encoder: e, ..
            }) => {
                size = Some((video.width, video.height));
                encoder = e;
            }
            Ok(MediaEvent::Packet(p)) if p.track == Track::Video => frames += 1,
            Ok(MediaEvent::Failed(message)) => {
                failure = Some(message);
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let stopper = std::thread::spawn(move || session.stop());
    while !stopper.is_finished() {
        rx.recv_timeout(Duration::from_millis(50)).ok();
    }
    stopper.join().ok();
    if let Some(message) = failure {
        return Err(message);
    }
    let (width, height) = size.ok_or("no Started event within 2 s")?;
    if frames == 0 {
        return Err("no frames received".into());
    }
    Ok((frames, width, height, encoder))
}

#[cfg(windows)]
fn deep_checks(out: &Path) -> bool {
    use greyscreen_capture::selftest;
    use greyscreen_core::encoder::EncoderKind;

    println!("deep check: encoding 300 frames per adapter, this takes a minute");
    let cfg = Settings::load(&paths::settings_file()).capture_config(NATIVE_HEIGHT_CEILING);
    let observations: Vec<_> = selftest::deep(&cfg)
        .into_iter()
        .map(|found| {
            let blank = deep::Observation {
                adapter: found.adapter,
                vendor_id: found.vendor_id,
                luid: found.luid,
                driver_version: found.driver_version,
                hardware: found.kind == EncoderKind::Hardware,
                is_capture_adapter: found.is_capture_adapter,
                encoder: String::new(),
                properties: Vec::new(),
                cpu_readback: false,
                size: (0, 0),
                fps: 0,
                frames_in: 0,
                packets: Vec::new(),
                nv12: None,
                error: None,
            };
            match found.run {
                Err(error) => deep::Observation {
                    error: Some(error),
                    ..blank
                },
                Ok(run) => deep::Observation {
                    encoder: run.encoder,
                    properties: run.properties,
                    cpu_readback: run.cpu_readback,
                    size: run.size,
                    fps: run.fps,
                    frames_in: run.frames_in,
                    packets: run
                        .packets
                        .into_iter()
                        .map(|p| deep::Packet {
                            annexb: p.annexb,
                            pts_hns: p.pts_hns,
                            key: p.key,
                        })
                        .collect(),
                    nv12: run.nv12.map(|m| deep::Nv12Means {
                        luma: m.luma,
                        luma_left: m.luma_left,
                        luma_right: m.luma_right,
                        chroma_u: m.chroma_u,
                        chroma_v: m.chroma_v,
                    }),
                    ..blank
                },
            }
        })
        .collect();
    let os = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let report = deep::build(&observations, env!("CARGO_PKG_VERSION"), &os);
    for adapter in &report.adapters {
        let verdict = if adapter.pass { "OK" } else { "FAILED" };
        println!(
            "  {} ({}): {verdict}, {} B-frames, nv12 {}",
            adapter.adapter, adapter.kind, adapter.b_frames, adapter.nv12.status
        );
        for failure in &adapter.failures {
            println!("    {failure}");
        }
    }
    if report.open_bframe_bug {
        println!("an encoder reordered frames: file the B-frame bug and send the report");
    }
    match write_report(&report, out) {
        Ok(()) => println!("deep report: {}", out.display()),
        Err(error) => println!("deep report: FAILED to write {}: {error}", out.display()),
    }
    report.pass
}

#[cfg(windows)]
fn write_report(report: &deep::Report, out: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    std::fs::write(out, json).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn deep_checks(_out: &Path) -> bool {
    println!("deep checks run on Windows only");
    true
}

#[cfg(not(windows))]
fn capture_checks() -> ExitCode {
    println!("capture checks run on Windows only");
    ExitCode::SUCCESS
}
