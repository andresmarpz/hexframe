use std::path::PathBuf;

const APP_DIR: &str = "Greyscreen";
pub(crate) const LEGACY_APP_DIR: &str = "Greyscreen";

pub(crate) fn config_root() -> PathBuf {
    dirs::config_dir().unwrap_or_default()
}

pub(crate) fn local_root() -> PathBuf {
    dirs::data_local_dir().unwrap_or_default()
}

pub(crate) fn videos_root() -> PathBuf {
    dirs::video_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Videos"))
}

pub(crate) fn config_dir() -> PathBuf {
    config_root().join(APP_DIR)
}

pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

pub fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir()
}

pub(crate) fn local_dir() -> PathBuf {
    local_root().join(APP_DIR)
}

pub fn logs_dir() -> PathBuf {
    local_dir().join("logs")
}

pub(crate) fn cache_dir() -> PathBuf {
    local_dir().join("cache")
}

pub fn champion_icons_dir() -> PathBuf {
    cache_dir().join("champions")
}

pub fn sounds_dir() -> PathBuf {
    cache_dir().join("sounds")
}

pub fn encoder_check_file() -> PathBuf {
    cache_dir().join("encoder-check.json")
}

pub(crate) fn default_output_dir() -> PathBuf {
    videos_root().join(APP_DIR)
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}
