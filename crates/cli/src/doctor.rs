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

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();
    if let Err(err) = probe::init_process() {
        println!("dpi awareness: {err}");
    }
    print!("{}", probe::system_report());
    let result = (|| -> anyhow::Result<String> {
        let _com = ComGuard::mta()?;
        let _mf = MfGuard::start()?;
        let gpu = GpuDevice::for_primary_monitor()?;
        Ok(H264Encoder::probe(&gpu)?)
    })();
    match result {
        Ok(name) => {
            println!("encoder: {name}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            println!("encoder: FAILED {err:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn capture_checks() -> ExitCode {
    println!("capture checks run on Windows only");
    ExitCode::SUCCESS
}
