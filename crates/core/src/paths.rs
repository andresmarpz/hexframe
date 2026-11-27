use std::path::PathBuf;

const APP_DIR: &str = "Greyscreen";

pub(crate) fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join(APP_DIR)
}

pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

pub fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir()
}

pub fn logs_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join(APP_DIR)
        .join("logs")
}

pub(crate) fn cache_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join(APP_DIR)
        .join("cache")
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
    match dirs::video_dir() {
        Some(videos) => videos.join(APP_DIR),
        None => dirs::home_dir()
            .unwrap_or_default()
            .join("Videos")
            .join(APP_DIR),
    }
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}
