use std::path::Path;

#[cfg(windows)]
pub fn copy_file(path: &Path) -> Result<(), String> {
    greyscreen_capture::clipboard::copy_file(path).map_err(|err| err.to_string())
}

#[cfg(target_os = "macos")]
pub fn copy_file(path: &Path) -> Result<(), String> {
    let quoted = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let script = format!("set the clipboard to (POSIX file \"{quoted}\")");
    let output = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn copy_file(_path: &Path) -> Result<(), String> {
    Err("copying files is not supported on this platform".into())
}
