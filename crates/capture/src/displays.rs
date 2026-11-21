use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};
use windows::core::BOOL;

pub fn connected_sizes() -> Vec<(u32, u32)> {
    let mut sizes: Vec<(u32, u32)> = Vec::new();
    // SAFETY: the callback only runs during this call and `sizes` outlives it.
    let listed = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut sizes as *mut Vec<(u32, u32)> as isize),
        )
    };
    if !listed.as_bool() {
        tracing::warn!("cannot list the connected displays");
        return Vec::new();
    }
    sizes
}

unsafe extern "system" fn collect(
    _monitor: HMONITOR,
    _dc: HDC,
    rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // SAFETY: `data` is the `Vec` that `connected_sizes` passed and keeps alive.
    let sizes = unsafe { &mut *(data.0 as *mut Vec<(u32, u32)>) };
    // SAFETY: Windows passes a valid monitor rectangle to the callback.
    let rect = unsafe { *rect };
    let width = rect.right.saturating_sub(rect.left).max(0) as u32;
    let height = rect.bottom.saturating_sub(rect.top).max(0) as u32;
    sizes.push((width, height));
    BOOL(1)
}
