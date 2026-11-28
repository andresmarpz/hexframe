use crate::encoder::{BUSY_HELP, OpenFailure};

pub const FAQ_URL: &str = "https://greyscreen.app/faq";
pub const NO_FRAMES: &str = "no frames from the game window";
pub const DISK_FULL_NOTICE: &str =
    "The disk is full, so the full game video stopped. Free some space to record the next game.";
pub const WRITE_ERROR_NOTICE: &str =
    "Writing the game video failed, so it stopped. Check the recordings drive.";
pub const FOLDER_UNUSABLE: &str = "recordings folder is not usable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    ExclusiveFullscreen,
    LeagueNotFound,
    FolderUnusable,
    LowDisk,
    NoEncoder,
    NoGpu,
    ConversionUnsupported,
    WindowsTooOld,
    CaptureBlocked,
    Other,
}

impl Problem {
    pub const ALL: [Problem; 10] = [
        Problem::ExclusiveFullscreen,
        Problem::LeagueNotFound,
        Problem::FolderUnusable,
        Problem::LowDisk,
        Problem::NoEncoder,
        Problem::NoGpu,
        Problem::ConversionUnsupported,
        Problem::WindowsTooOld,
        Problem::CaptureBlocked,
        Problem::Other,
    ];

    pub fn from_error(raw: &str) -> Problem {
        let raw = raw.to_ascii_lowercase();
        let has = |needles: &[&str]| needles.iter().any(|needle| raw.contains(needle));
        if has(&[NO_FRAMES]) {
            Problem::ExclusiveFullscreen
        } else if has(&[
            "not enough space",
            "no space left",
            "disk is full",
            "no storage space",
            "os error 112",
        ]) {
            Problem::LowDisk
        } else if has(&[FOLDER_UNUSABLE, "os error 5)"]) {
            Problem::FolderUnusable
        } else if has(&["graphics capture is not supported"]) {
            Problem::WindowsTooOld
        } else if has(&["no usable gpu"]) {
            Problem::NoGpu
        } else if has(&["bgra to nv12"]) {
            Problem::ConversionUnsupported
        } else if has(&["video encoder failed", "h.264 encoder"]) {
            Problem::NoEncoder
        } else if has(&["window closed", "invalid window handle"]) {
            Problem::LeagueNotFound
        } else if has(&["access is denied", "0x80070005"]) {
            Problem::CaptureBlocked
        } else {
            Problem::Other
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Problem::ExclusiveFullscreen => "League is in Fullscreen",
            Problem::LeagueNotFound => "League window not found",
            Problem::FolderUnusable => "Can't save to the recordings folder",
            Problem::LowDisk => "Not enough disk space",
            Problem::NoEncoder => "No video encoder found",
            Problem::NoGpu => "No graphics card found",
            Problem::ConversionUnsupported => "Graphics card cannot convert video",
            Problem::WindowsTooOld => "Windows is too old to record",
            Problem::CaptureBlocked => "Windows blocked screen capture",
            Problem::Other => "Capture failed",
        }
    }

    pub fn action(self) -> &'static str {
        match self {
            Problem::ExclusiveFullscreen => {
                "Switch League to Borderless in its video settings, then retry."
            }
            Problem::LeagueNotFound => "Make sure League is running, then retry.",
            Problem::FolderUnusable => "Choose another recordings folder.",
            Problem::LowDisk => "Free some space or choose another folder, then retry.",
            Problem::NoEncoder => "Update your graphics driver, then restart Greyscreen.",
            Problem::NoGpu => {
                "Check that your graphics card is enabled and its driver is installed, then restart Greyscreen."
            }
            Problem::ConversionUnsupported => {
                "Update your graphics driver, then restart Greyscreen."
            }
            Problem::WindowsTooOld => "Update to Windows 10 version 1903 or newer.",
            Problem::CaptureBlocked => {
                "If League runs as administrator, run Greyscreen as administrator too."
            }
            Problem::Other => "Retry. If it keeps failing, send feedback.",
        }
    }

    pub fn also(self) -> Option<&'static str> {
        match self {
            Problem::ExclusiveFullscreen => Some(
                "Still no picture? If League runs as administrator, run Greyscreen as administrator too.",
            ),
            Problem::CaptureBlocked => {
                Some("Otherwise check Windows Settings > Privacy & security > Screen capture.")
            }
            _ => None,
        }
    }

    pub fn anchor(self) -> &'static str {
        match self {
            Problem::ExclusiveFullscreen => "fullscreen",
            Problem::LeagueNotFound => "window-not-found",
            Problem::FolderUnusable => "folder-not-writable",
            Problem::LowDisk => "low-disk",
            Problem::NoEncoder => "no-encoder",
            Problem::NoGpu => "no-gpu",
            Problem::ConversionUnsupported => "video-conversion",
            Problem::WindowsTooOld => "windows-version",
            Problem::CaptureBlocked => "run-as-admin",
            Problem::Other => "help",
        }
    }

    pub fn help_url(self) -> String {
        format!("{FAQ_URL}#{}", self.anchor())
    }

    pub fn status_title(self) -> &'static str {
        match self {
            Problem::ExclusiveFullscreen => "League: Fullscreen",
            Problem::LeagueNotFound => "League not found",
            Problem::FolderUnusable => "Folder not usable",
            Problem::LowDisk => "Disk is full",
            Problem::NoEncoder => "No video encoder",
            Problem::NoGpu => "No graphics card",
            Problem::ConversionUnsupported => "Conversion failed",
            Problem::WindowsTooOld => "Windows too old",
            Problem::CaptureBlocked => "Capture blocked",
            Problem::Other => "Capture failed",
        }
    }

    pub fn status_detail(self) -> &'static str {
        match self {
            Problem::ExclusiveFullscreen => "Switch to Borderless",
            Problem::LeagueNotFound => "Start a game",
            Problem::FolderUnusable => "Choose another folder",
            Problem::LowDisk => "Free some space",
            Problem::NoEncoder => "Update graphics driver",
            Problem::NoGpu => "Check your GPU",
            Problem::ConversionUnsupported => "Update graphics driver",
            Problem::WindowsTooOld => "Update Windows",
            Problem::CaptureBlocked => "Check Windows settings",
            Problem::Other => "Open Home to retry",
        }
    }

    pub fn retry_helps(self) -> bool {
        !matches!(
            self,
            Problem::FolderUnusable
                | Problem::LowDisk
                | Problem::NoEncoder
                | Problem::NoGpu
                | Problem::ConversionUnsupported
                | Problem::WindowsTooOld
        )
    }

    pub fn offers_folder(self) -> bool {
        matches!(self, Problem::FolderUnusable | Problem::LowDisk)
    }

    pub fn write_failure_notice(raw: &str) -> &'static str {
        if Problem::from_error(raw) == Problem::LowDisk {
            DISK_FULL_NOTICE
        } else {
            WRITE_ERROR_NOTICE
        }
    }

    pub fn notice(prefix: &str, raw: &str) -> String {
        match Problem::from_error(raw) {
            Problem::Other => format!("{prefix}: {raw}"),
            Problem::NoEncoder if OpenFailure::from_text(raw) == OpenFailure::Busy => {
                BUSY_HELP.to_owned()
            }
            problem => format!("{}. {}", problem.title(), problem.action()),
        }
    }
}

#[cfg(test)]
mod tests;
