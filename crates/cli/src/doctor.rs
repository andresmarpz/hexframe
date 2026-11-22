use std::process::ExitCode;

use greyscreen_core::paths;
#[cfg(windows)]
use greyscreen_core::settings::NATIVE_HEIGHT_CEILING;
use greyscreen_core::settings::Settings;

pub(crate) fn run() -> ExitCode {
    print_environment();
    capture_checks()
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

#[cfg(not(windows))]
fn capture_checks() -> ExitCode {
    println!("capture checks run on Windows only");
    ExitCode::SUCCESS
}
