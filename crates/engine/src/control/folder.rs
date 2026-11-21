use std::fs;
use std::path::Path;

use greyscreen_core::problem::FOLDER_UNUSABLE;

const PROBE_FILE: &str = ".greyscreen-write-check";

pub(super) fn check_writable(dir: &Path) -> Result<(), String> {
    let unusable = |err: std::io::Error| format!("{FOLDER_UNUSABLE}: {}: {err}", dir.display());
    fs::create_dir_all(dir).map_err(unusable)?;
    let probe = dir.join(PROBE_FILE);
    fs::write(&probe, b"").map_err(unusable)?;
    fs::remove_file(&probe).map_err(unusable)
}

#[cfg(test)]
mod tests;
