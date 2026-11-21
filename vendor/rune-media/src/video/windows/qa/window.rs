//! A window for the clip's layer that no one sees, chosen by `RUNE_MEDIA_QA_WINDOW`:
//!
//! - unset or `none`: no window. The layer's surface is on no visual and nothing composes it.
//! - `hidden`: a 1x1 tool window, cloaked and never shown, whose DirectComposition target shows
//!   the layer.
//! - `cloaked`: the same window cloaked first (`DWMWA_CLOAK`, documented as "not visible to the
//!   user" and "still composed by DWM"), then shown without activation. Creation fails, and
//!   nothing shows, if the cloak doesn't read back or the foreground window changes.
//!
//! The window never takes input: it is a tool window (no taskbar button, no Alt-Tab), refuses
//! activation and has no redirection surface. Nothing here sends input.

use super::super::mf;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::Graphics::DirectComposition::{IDCompositionDevice, IDCompositionTarget, IDCompositionVisual};
use windows::core::{GUID, HRESULT, IUnknown, Interface};
use windows_sys::Win32::Graphics::Dwm::{DWM_CLOAKED_APP, DWMWA_CLOAK, DWMWA_CLOAKED, DwmGetWindowAttribute, DwmSetWindowAttribute};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    None,
    Hidden,
    Cloaked,
}

pub(super) fn mode() -> Mode {
    match std::env::var("RUNE_MEDIA_QA_WINDOW").as_deref() {
        Ok("hidden") => Mode::Hidden,
        Ok("cloaked") => Mode::Cloaked,
        _ => Mode::None,
    }
}

type CreateDevice = unsafe extern "system" fn(*mut core::ffi::c_void, *const GUID, *mut *mut core::ffi::c_void) -> HRESULT;

/// The composition device and visual tree are used from one test thread at a time.
struct Composition {
    device: IDCompositionDevice,
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
}

// SAFETY: DirectComposition objects may be used from any thread, one at a time.
unsafe impl Send for Composition {}

pub(super) struct Host {
    hwnd: isize,
    pump: Option<JoinHandle<()>>,
    composition: Composition,
    shown: Option<usize>,
}

impl Host {
    /// The window for `mode`; None for `Mode::None`.
    pub(super) fn open(mode: Mode) -> Result<Option<Host>, String> {
        if mode == Mode::None {
            return Ok(None);
        }
        let (made, hwnd) = mpsc::sync_channel(1);
        let pump = std::thread::Builder::new()
            .name("rune-media-qa-window".into())
            .spawn(move || {
                // SAFETY: a window of this thread, pumped below until it is destroyed.
                match unsafe { make(mode) } {
                    Ok(hwnd) => {
                        let _ = made.send(Ok(hwnd));
                        // SAFETY: this thread's message loop.
                        unsafe { pump() };
                    }
                    Err(e) => {
                        let _ = made.send(Err(e));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let hwnd = hwnd.recv().map_err(|_| "the window thread stopped".to_string())??;
        let composition = compose(hwnd).inspect_err(|_| close(hwnd))?;
        Ok(Some(Host { hwnd, pump: Some(pump), composition, shown: None }))
    }

    /// Puts the layer's surface `handle` on the window's visual and commits. Returns whether DWM
    /// took the commit within a second.
    pub(super) fn show(&mut self, handle: usize) -> Result<bool, String> {
        if self.shown == Some(handle) {
            return Ok(true);
        }
        let c = &self.composition;
        // SAFETY: COM calls on this window's device; the decoder keeps the handle open while its
        // layer lives.
        unsafe {
            let surface: IUnknown = c.device.CreateSurfaceFromHandle(HANDLE(handle as *mut _)).map_err(|e| format!("CreateSurfaceFromHandle: {e}"))?;
            c.visual.SetContent(&surface).map_err(|e| format!("SetContent: {e}"))?;
            c.device.Commit().map_err(|e| format!("Commit: {e}"))?;
        }
        self.shown = Some(handle);
        let device = Composition { device: c.device.clone(), _target: c._target.clone(), visual: c.visual.clone() };
        let (done, waited) = mpsc::sync_channel(1);
        // A commit DWM never takes would block this wait for good: it gets its own thread.
        let _ = std::thread::Builder::new().name("rune-media-qa-commit".into()).spawn(move || {
            let composition = device;
            // SAFETY: a COM call on the window's device, alone on this thread.
            let _ = done.send(unsafe { composition.device.WaitForCommitCompletion() }.is_ok());
        });
        Ok(waited.recv_timeout(Duration::from_secs(1)).unwrap_or(false))
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        close(self.hwnd);
        if let Some(pump) = self.pump.take() {
            let _ = pump.join();
        }
    }
}

fn close(hwnd: isize) {
    // SAFETY: asks the window's own thread to destroy it.
    unsafe { PostMessageW(hwnd as SysHwnd, WM_CLOSE, 0, 0) };
}

type SysHwnd = windows_sys::Win32::Foundation::HWND;

fn compose(hwnd: isize) -> Result<Composition, String> {
    let dcomp = mf::system_library("dcomp.dll").ok_or("dcomp.dll is missing")?;
    // SAFETY: the documented entry point at its documented signature; the pointer it writes is an
    // owned reference. The target is for a window of this process.
    unsafe {
        let create: CreateDevice = mf::symbol(dcomp, c"DCompositionCreateDevice2").ok_or("no DCompositionCreateDevice2")?;
        let mut raw = std::ptr::null_mut();
        create(std::ptr::null_mut(), &IDCompositionDevice::IID, &mut raw).ok().map_err(|e| format!("DCompositionCreateDevice2: {e}"))?;
        let device = IDCompositionDevice::from_raw(raw);
        let target = device.CreateTargetForHwnd(HWND(hwnd as *mut _), false).map_err(|e| format!("CreateTargetForHwnd: {e}"))?;
        let visual = device.CreateVisual().map_err(|e| format!("CreateVisual: {e}"))?;
        target.SetRoot(&visual).map_err(|e| format!("SetRoot: {e}"))?;
        Ok(Composition { device, _target: target, visual })
    }
}

/// A cloaked 1x1 tool window, shown without activation for `Mode::Cloaked`.
unsafe fn make(mode: Mode) -> Result<isize, String> {
    let class: Vec<u16> = "rune-media-qa\0".encode_utf16().collect();
    // SAFETY: Win32 calls on a window this thread makes; every pointer outlives its call.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: instance, lpszClassName: class.as_ptr(), ..std::mem::zeroed() };
        // A second test registers the class again and fails harmlessly.
        RegisterClassW(&wc);
        let ex = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_NOREDIRECTIONBITMAP;
        let hwnd = CreateWindowExW(ex, class.as_ptr(), class.as_ptr(), WS_POPUP, 0, 0, 1, 1, std::ptr::null_mut(), std::ptr::null_mut(), instance, std::ptr::null());
        if hwnd.is_null() {
            return Err("CreateWindowExW failed".into());
        }
        let fail = |e: &str| {
            DestroyWindow(hwnd);
            Err(e.to_string())
        };
        let on: i32 = 1;
        if DwmSetWindowAttribute(hwnd, DWMWA_CLOAK as u32, (&raw const on).cast(), 4) < 0 {
            return fail("DWMWA_CLOAK was refused");
        }
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as u32, (&raw mut cloaked).cast(), 4) < 0 || cloaked & DWM_CLOAKED_APP == 0 {
            return fail("the window didn't read back as cloaked");
        }
        if mode == Mode::Cloaked {
            // A process's first ShowWindow may take its launcher's show command instead.
            ShowWindow(hwnd, SW_HIDE);
            let before = GetForegroundWindow();
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            if GetForegroundWindow() != before {
                return fail("showing the cloaked window changed the foreground window");
            }
        }
        Ok(hwnd as isize)
    }
}

unsafe extern "system" fn proc(hwnd: SysHwnd, msg: u32, w: usize, l: isize) -> isize {
    // SAFETY: the default handling for this thread's own window.
    unsafe {
        match msg {
            WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, w, l),
        }
    }
}

unsafe fn pump() {
    // SAFETY: the standard message loop of this thread.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
