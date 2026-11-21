use std::fs::OpenOptions;
use std::os::windows::io::IntoRawHandle;
use std::path::Path;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};

pub fn send_rune_video_log_to(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    // SAFETY: main calls this before it starts any other thread, so nothing reads the environment at the same time.
    unsafe { std::env::set_var("RUNE_VIDEO_LOG", "frames") };
    let handle = HANDLE(file.into_raw_handle());
    // SAFETY: the handle is an open file that the process keeps until it exits.
    unsafe { SetStdHandle(STD_ERROR_HANDLE, handle) }.map_err(std::io::Error::other)
}
