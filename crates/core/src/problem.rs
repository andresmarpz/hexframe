pub const FAQ_URL: &str = "https://greyscreen-lake.vercel.app/faq";
pub const NO_FRAMES: &str = "no frames from the game window";
pub const FOLDER_UNUSABLE: &str = "recordings folder is not usable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    ExclusiveFullscreen,
    LeagueNotFound,
    FolderUnusable,
    LowDisk,
    NoEncoder,
    WindowsTooOld,
    CaptureBlocked,
    Other,
}

impl Problem {
    pub const ALL: [Problem; 8] = [
        Problem::ExclusiveFullscreen,
        Problem::LeagueNotFound,
        Problem::FolderUnusable,
        Problem::LowDisk,
        Problem::NoEncoder,
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
            "os error 112",
        ]) {
            Problem::LowDisk
        } else if has(&[FOLDER_UNUSABLE]) {
            Problem::FolderUnusable
        } else if has(&["graphics capture is not supported"]) {
            Problem::WindowsTooOld
        } else if has(&["video encoder failed", "h.264 encoder", "no usable gpu"]) {
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
            Problem::LeagueNotFound => "league-not-found",
            Problem::FolderUnusable => "recordings-folder",
            Problem::LowDisk => "low-disk",
            Problem::NoEncoder => "no-encoder",
            Problem::WindowsTooOld => "windows-version",
            Problem::CaptureBlocked => "capture-blocked",
            Problem::Other => "capture-failed",
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
                | Problem::WindowsTooOld
        )
    }

    pub fn offers_folder(self) -> bool {
        matches!(self, Problem::FolderUnusable | Problem::LowDisk)
    }

    pub fn notice(prefix: &str, raw: &str) -> String {
        match Problem::from_error(raw) {
            Problem::Other => format!("{prefix}: {raw}"),
            problem => format!("{}. {}", problem.title(), problem.action()),
        }
    }
}

#[cfg(test)]
mod tests;
