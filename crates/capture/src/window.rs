use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};
use windows::core::BOOL;

struct Search {
    needle: String,
    found: Option<(isize, u32)>,
}

pub fn find_by_title(needle: &str) -> Option<(isize, u32)> {
    let mut search = Search {
        needle: needle.to_lowercase(),
        found: None,
    };
    // SAFETY: the callback only touches `search`, which outlives the synchronous call.
    unsafe {
        EnumWindows(
            Some(enum_proc),
            LPARAM(std::ptr::from_mut(&mut search) as isize),
        )
    }
    .ok();
    search.found
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam is the `Search` pointer passed by `find_by_title` for this call.
    let search = unsafe { &mut *(lparam.0 as *mut Search) };
    // SAFETY: hwnd comes from EnumWindows.
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return BOOL(1);
    }
    let mut buf = [0u16; 512];
    // SAFETY: the buffer is a live local slice.
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    let len = usize::try_from(len).unwrap_or(0);
    let title = String::from_utf16_lossy(&buf[..len]).to_lowercase();
    if len == 0 || !title.contains(&search.needle) {
        return BOOL(1);
    }
    let mut pid = 0u32;
    // SAFETY: `pid` is a live local that receives the process id.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    search.found = Some((hwnd.0 as isize, pid));
    BOOL(0)
}
