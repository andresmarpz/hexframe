use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const APP_ID: &str = "dev.greyscreen.app";
pub const EXE_NAME: &str = "greyscreen.exe";
pub const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Greyscreen";
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const RUN_VALUE: &str = "Greyscreen";
pub const UNINSTALL_FLAG: &str = "--uninstall";
pub const HIDDEN_FLAG: &str = "--hidden";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Text(String),
    Number(u32),
}

pub fn is_inside(exe: &Path, dir: &Path) -> bool {
    lowercase(exe).starts_with(lowercase(dir))
}

fn lowercase(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

pub fn run_command(exe: &Path) -> String {
    format!("\"{}\" {HIDDEN_FLAG}", exe.display())
}

pub fn uninstall_command(exe: &Path) -> String {
    format!("\"{}\" {UNINSTALL_FLAG}", exe.display())
}

pub fn uninstall_values(
    exe: &Path,
    dir: &Path,
    version: &str,
    size_bytes: u64,
) -> Vec<(&'static str, RegValue)> {
    let text = |value: &str| RegValue::Text(value.to_owned());
    vec![
        ("DisplayName", text("Greyscreen")),
        ("DisplayVersion", text(version)),
        ("Publisher", text("Greyscreen")),
        ("DisplayIcon", text(&exe.display().to_string())),
        ("InstallLocation", text(&dir.display().to_string())),
        ("UninstallString", text(&uninstall_command(exe))),
        ("NoModify", RegValue::Number(1)),
        ("NoRepair", RegValue::Number(1)),
        (
            "EstimatedSize",
            RegValue::Number(u32::try_from(size_bytes.div_ceil(1024)).unwrap_or(u32::MAX)),
        ),
    ]
}

pub const QUIT_WAIT: Duration = Duration::from_secs(10);
pub const QUIT_POLL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPlan {
    CopyFresh,
    QuitAndReplace,
    RelaunchInstalled,
}

pub fn install_plan(installed_exists: bool, installed_matches_running: bool) -> InstallPlan {
    match (installed_exists, installed_matches_running) {
        (false, _) => InstallPlan::CopyFresh,
        (true, true) => InstallPlan::RelaunchInstalled,
        (true, false) => InstallPlan::QuitAndReplace,
    }
}

pub fn wait_until(timeout: Duration, poll: Duration, mut ready: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    loop {
        if ready() {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        std::thread::sleep(poll);
    }
}

pub fn cleanup_command(dir: &Path) -> String {
    format!(
        "ping 127.0.0.1 -n 3 >nul & rmdir /s /q \"{}\"",
        dir.display()
    )
}

#[cfg(test)]
mod tests;
