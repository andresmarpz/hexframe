use std::process::ExitCode;

use greyscreen_core::paths;
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
    use greyscreen_capture::com::{ComGuard, MfGuard};
    use greyscreen_capture::d3d::GpuDevice;
    use greyscreen_capture::h264::H264Encoder;
    use greyscreen_capture::probe;

    crate::init_windows();
    print!("{}", probe::system_report());
    let result = (|| -> anyhow::Result<String> {
        let _com = ComGuard::mta()?;
        let _mf = MfGuard::start()?;
        let gpu = GpuDevice::for_primary_monitor()?;
        Ok(H264Encoder::probe(&gpu)?)
    })();
    let encoder_ok = match result {
        Ok(name) => {
            println!("encoder: {name}");
            true
        }
        Err(err) => {
            println!("encoder: FAILED {err:#}");
            false
        }
    };
    let capture_ok = match capture_test() {
        Ok((frames, width, height)) => {
            println!("capture: OK, {frames} frames, {width}x{height}");
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
fn capture_test() -> Result<(u64, u32, u32), String> {
    use std::time::{Duration, Instant};

    use greyscreen_capture::WindowsMediaSource;
    use greyscreen_core::media::{CaptureTarget, MediaEvent, MediaSource, Track};

    const TEST_DURATION: Duration = Duration::from_secs(2);

    let mut cfg = Settings::default().capture_config(2160);
    cfg.fps = 30;
    cfg.audio = false;
    let (tx, rx) = crossbeam_channel::bounded(1024);
    let session = WindowsMediaSource::new().start(CaptureTarget::PrimaryMonitor, cfg, tx)?;
    let deadline = Instant::now() + TEST_DURATION;
    let mut size = None;
    let mut frames = 0u64;
    let mut failure = None;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(MediaEvent::Started { video, .. }) => size = Some((video.width, video.height)),
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
    Ok((frames, width, height))
}

#[cfg(not(windows))]
fn capture_checks() -> ExitCode {
    println!("capture checks run on Windows only");
    ExitCode::SUCCESS
}
