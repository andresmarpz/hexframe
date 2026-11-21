use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, POINT};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
};
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::UI::Shell::DROPFILES;
use windows::core::BOOL;

const OPEN_TRIES: u32 = 20;
const OPEN_WAIT: Duration = Duration::from_millis(10);

struct OpenClipboardGuard;

impl OpenClipboardGuard {
    fn open() -> io::Result<Self> {
        for _ in 0..OPEN_TRIES {
            // SAFETY: a null owner window is allowed; the matching CloseClipboard runs on drop.
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(Self);
            }
            sleep(OPEN_WAIT);
        }
        Err(io::Error::other(
            "another program is using the clipboard, try again",
        ))
    }
}

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: balances the successful OpenClipboard in `open` on this thread.
        unsafe { CloseClipboard() }.ok();
    }
}

fn windows_error(err: &windows::core::Error) -> io::Error {
    io::Error::other(err.message())
}

fn drop_list(path: &Path) -> io::Result<Vec<u8>> {
    let absolute = std::path::absolute(path)?;
    let mut wide: Vec<u16> = absolute.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::other("the path has an empty character"));
    }
    wide.extend([0, 0]);
    let header = DROPFILES {
        pFiles: size_of::<DROPFILES>() as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: BOOL(0),
        fWide: BOOL(1),
    };
    let mut bytes = Vec::with_capacity(size_of::<DROPFILES>() + wide.len() * 2);
    bytes.extend_from_slice(&header.pFiles.to_le_bytes());
    bytes.extend_from_slice(&header.pt.x.to_le_bytes());
    bytes.extend_from_slice(&header.pt.y.to_le_bytes());
    bytes.extend_from_slice(&header.fNC.0.to_le_bytes());
    bytes.extend_from_slice(&header.fWide.0.to_le_bytes());
    for unit in wide {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    Ok(bytes)
}

fn global_copy(bytes: &[u8]) -> io::Result<HGLOBAL> {
    // SAFETY: allocates a movable, zeroed block of exactly `bytes.len()` bytes.
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len()) }
        .map_err(|err| windows_error(&err))?;
    // SAFETY: `memory` was just allocated and is unlocked after the copy.
    let base = unsafe { GlobalLock(memory) };
    if base.is_null() {
        // SAFETY: the block is unused and freed once.
        unsafe { GlobalFree(Some(memory)) }.ok();
        return Err(io::Error::other("cannot lock the clipboard memory"));
    }
    // SAFETY: the locked block holds `bytes.len()` bytes and does not overlap `bytes`.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.cast::<u8>(), bytes.len());
        GlobalUnlock(memory).ok();
    }
    Ok(memory)
}

pub fn copy_file(path: &Path) -> io::Result<()> {
    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the file is not there any more",
        ));
    }
    let memory = global_copy(&drop_list(path)?)?;
    let result = (|| {
        let _open = OpenClipboardGuard::open()?;
        // SAFETY: the clipboard is open on this thread.
        unsafe { EmptyClipboard() }.map_err(|err| windows_error(&err))?;
        // SAFETY: the clipboard is open and `memory` is a movable block; on success the system owns it.
        unsafe { SetClipboardData(u32::from(CF_HDROP.0), Some(HANDLE(memory.0))) }
            .map(drop)
            .map_err(|err| windows_error(&err))
    })();
    if result.is_err() {
        // SAFETY: the system did not take the block, so it is freed once here.
        unsafe { GlobalFree(Some(memory)) }.ok();
    }
    result
}
