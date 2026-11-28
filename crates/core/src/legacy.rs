use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::paths::{self, LEGACY_APP_DIR};
use crate::settings::Settings;

const SETTINGS_FILE: &str = "settings.json";
const OUTPUT_DIR_KEY: &str = "output_dir";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adoption {
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },
    SettingsCopied {
        from: PathBuf,
        to: PathBuf,
        rename_error: String,
    },
    Failed {
        from: PathBuf,
        to: PathBuf,
        error: String,
    },
    RecordingsKept {
        dir: PathBuf,
    },
}

impl Adoption {
    pub fn log(&self) {
        match self {
            Adoption::Renamed { from, to } => {
                tracing::info!(from = %from.display(), to = %to.display(), "moved the old Greyscreen folder");
            }
            Adoption::SettingsCopied {
                from,
                to,
                rename_error,
            } => {
                tracing::warn!(from = %from.display(), to = %to.display(), error = %rename_error, "cannot move the old Greyscreen folder, copied its settings and left it in place");
            }
            Adoption::Failed { from, to, error } => {
                tracing::warn!(from = %from.display(), to = %to.display(), %error, "cannot move the old Greyscreen folder, left it in place");
            }
            Adoption::RecordingsKept { dir } => {
                tracing::info!(dir = %dir.display(), "kept the old Greyscreen recordings folder as the recordings folder");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirPlan {
    Adopt,
    Leave,
}

fn dir_plan(new_exists: bool, old_exists: bool) -> DirPlan {
    if !new_exists && old_exists {
        DirPlan::Adopt
    } else {
        DirPlan::Leave
    }
}

pub fn adopt_legacy_data() -> Vec<Adoption> {
    let mut done = Vec::new();
    let config = paths::config_root();
    let local = paths::local_root();
    done.extend(adopt_dir(
        &config.join(LEGACY_APP_DIR),
        &paths::config_dir(),
    ));
    if local != config {
        done.extend(adopt_dir(&local.join(LEGACY_APP_DIR), &paths::local_dir()));
    }
    let videos = paths::videos_root();
    done.extend(keep_legacy_recordings(
        &paths::settings_file(),
        &videos.join(LEGACY_APP_DIR),
        &paths::default_output_dir(),
    ));
    done
}

fn adopt_dir(old: &Path, new: &Path) -> Option<Adoption> {
    if old.as_os_str().is_empty() || dir_plan(new.exists(), old.exists()) == DirPlan::Leave {
        return None;
    }
    let (from, to) = (old.to_owned(), new.to_owned());
    Some(match fs::rename(old, new) {
        Ok(()) => Adoption::Renamed { from, to },
        Err(rename_error) => match copy_settings(old, new) {
            Ok(()) => Adoption::SettingsCopied {
                from,
                to,
                rename_error: rename_error.to_string(),
            },
            Err(_) => Adoption::Failed {
                from,
                to,
                error: rename_error.to_string(),
            },
        },
    })
}

fn copy_settings(old: &Path, new: &Path) -> io::Result<()> {
    let source = old.join(SETTINGS_FILE);
    if !source.is_file() {
        return Err(io::ErrorKind::NotFound.into());
    }
    fs::create_dir_all(new)?;
    fs::copy(source, new.join(SETTINGS_FILE)).map(drop)
}

fn keep_legacy_recordings(
    settings_file: &Path,
    legacy_videos: &Path,
    new_videos: &Path,
) -> Option<Adoption> {
    if legacy_videos.as_os_str().is_empty() || !legacy_videos.is_dir() || new_videos.exists() {
        return None;
    }
    let text = match fs::read_to_string(settings_file) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(_) => return None,
    };
    let updated = with_explicit_output_dir(text.as_deref(), legacy_videos)?;
    write_atomic(settings_file, updated.as_bytes()).ok()?;
    Some(Adoption::RecordingsKept {
        dir: legacy_videos.to_owned(),
    })
}

fn with_explicit_output_dir(settings_json: Option<&str>, legacy_videos: &Path) -> Option<String> {
    let Some(text) = settings_json else {
        let settings = Settings {
            output_dir: legacy_videos.to_owned(),
            ..Settings::default()
        };
        return serde_json::to_string_pretty(&settings).ok();
    };
    let mut value: serde_json::Value = serde_json::from_str(text).ok()?;
    let object = value.as_object_mut()?;
    if object.contains_key(OUTPUT_DIR_KEY) {
        return None;
    }
    object.insert(
        OUTPUT_DIR_KEY.to_owned(),
        serde_json::to_value(legacy_videos).ok()?,
    );
    serde_json::to_string_pretty(&value).ok()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}
