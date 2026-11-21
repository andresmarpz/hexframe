use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::UI::Shell::{
    FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW,
    SHFileOperationW,
};
use windows::core::PCWSTR;

const IN_USE_CODES: [i32; 3] = [0x20, 0x21, 0x78];

pub fn move_to_recycle_bin(path: &Path) -> io::Result<()> {
    let mut from: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .map(|unit| {
            if unit == u16::from(b'/') {
                u16::from(b'\\')
            } else {
                unit
            }
        })
        .collect();
    from.extend([0, 0]);
    let flags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT;
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: flags.0 as u16,
        ..SHFILEOPSTRUCTW::default()
    };
    // SAFETY: `from` is double-null terminated and outlives the call, and `operation` is initialised.
    let code = unsafe { SHFileOperationW(&mut operation) };
    if code != 0 {
        if IN_USE_CODES.contains(&code) {
            return Err(io::Error::new(
                io::ErrorKind::ResourceBusy,
                format!("the file is in use (code {code:#x})"),
            ));
        }
        return Err(io::Error::other(format!(
            "the Recycle Bin refused the folder (code {code:#x})"
        )));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err(io::Error::other(
            "the move to the Recycle Bin was cancelled",
        ));
    }
    Ok(())
}
