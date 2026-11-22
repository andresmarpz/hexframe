mod pattern;

use self::pattern::{BAND_FROM_PERCENT, BAND_TO_PERCENT, BLACK, BLUE, GREEN, RED, WHITE, YELLOW};
pub use self::pattern::{COUNTER_BITS, Frame, FrameReader, PatternError, read_counter};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Sender, bounded};
use greyscreen_core::media::CaptureTarget;
use windows::Win32::Foundation::{
    COLORREF, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{
    DWMWA_CLOAK, DWMWA_CLOAKED, DwmGetWindowAttribute, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC,
    DeleteObject, EndPaint, FillRect, HBRUSH, HDC, InvalidateRect, PAINTSTRUCT, SRCCOPY,
    SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect,
    GetForegroundWindow, IsIconic, MSG, MsgWaitForMultipleObjects, PM_REMOVE, PeekMessageW,
    PostMessageW, PostQuitMessage, QS_ALLINPUT, RegisterClassW, SHOW_WINDOW_CMD, SW_HIDE,
    SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
    ShowWindow, TranslateMessage, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_PAINT, WM_QUIT,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{BOOL, w};

const TICK_EVERY: Duration = Duration::from_micros(16_667);
const READY_WAIT: Duration = Duration::from_secs(5);
const CLOSE_WAIT: Duration = Duration::from_secs(5);
const START_POSITION: i32 = 100;

// One window exists at a time, so the paint callback reads its counter from here.
static COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, thiserror::Error)]
pub enum CloakError {
    #[error("windows api error: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("the window thread did not report a window in time")]
    NoWindow,
    #[error("the cloak did not read back from the window")]
    CloakNotApplied,
    #[error("the foreground window changed while the window was shown")]
    ForegroundChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    NeverShown,
    ShownCloaked,
}

#[derive(Debug)]
pub struct CloakedWindow {
    hwnd: isize,
    foreground: isize,
    thread: Option<JoinHandle<()>>,
}

impl CloakedWindow {
    pub fn open(width: i32, height: i32, reveal: Reveal) -> Result<Self, CloakError> {
        COUNTER.store(0, Ordering::Relaxed);
        let (ready, wait) = bounded(1);
        let foreground = foreground_handle();
        let thread = std::thread::Builder::new()
            .name("cloaked-window".into())
            .spawn(move || run(width, height, reveal, foreground, &ready))
            .map_err(|_| CloakError::NoWindow)?;
        match wait.recv_timeout(READY_WAIT) {
            Ok(Ok(hwnd)) => Ok(Self {
                hwnd,
                foreground,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                thread.join().ok();
                Err(error)
            }
            Err(_) => Err(CloakError::NoWindow),
        }
    }

    pub fn hwnd(&self) -> isize {
        self.hwnd
    }

    pub fn target(&self) -> CaptureTarget {
        CaptureTarget::GameWindow {
            hwnd: self.hwnd,
            pid: std::process::id(),
        }
    }

    pub fn ticks(&self) -> u32 {
        COUNTER.load(Ordering::Relaxed)
    }

    pub fn is_cloaked(&self) -> bool {
        cloaked_flags(hwnd_of(self.hwnd)).is_ok_and(|flags| flags != 0)
    }

    pub fn is_minimised(&self) -> bool {
        // SAFETY: IsIconic accepts any window handle value.
        unsafe { IsIconic(hwnd_of(self.hwnd)) }.as_bool()
    }

    pub fn foreground_unchanged(&self) -> bool {
        foreground_handle() == self.foreground
    }

    pub fn client_size(&self) -> Option<(i32, i32)> {
        let mut rect = RECT::default();
        // SAFETY: `rect` is a live out value.
        unsafe { GetClientRect(hwnd_of(self.hwnd), &mut rect) }.ok()?;
        Some((rect.right - rect.left, rect.bottom - rect.top))
    }

    pub fn resize(&self, width: i32, height: i32) -> Result<(), CloakError> {
        // SAFETY: the window belongs to this process and the call only takes values.
        unsafe {
            SetWindowPos(
                hwnd_of(self.hwnd),
                None,
                0,
                0,
                width,
                height,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        }?;
        Ok(())
    }

    pub fn minimise(&self) {
        show(self.hwnd, SW_SHOWMINNOACTIVE);
    }

    pub fn restore(&self) {
        show(self.hwnd, SW_SHOWNOACTIVATE);
    }

    pub fn hide(&self) {
        show(self.hwnd, SW_HIDE);
    }

    pub fn reshow(&self) {
        show(self.hwnd, SW_SHOWNOACTIVATE);
    }

    pub fn close(mut self) -> bool {
        self.close_inner()
    }

    fn close_inner(&mut self) -> bool {
        let Some(thread) = self.thread.take() else {
            return true;
        };
        // SAFETY: posting WM_CLOSE to a window handle value is always allowed.
        unsafe { PostMessageW(Some(hwnd_of(self.hwnd)), WM_CLOSE, WPARAM(0), LPARAM(0)) }.ok();
        let started = Instant::now();
        while !thread.is_finished() && started.elapsed() < CLOSE_WAIT {
            std::thread::sleep(Duration::from_millis(5));
        }
        let finished = thread.is_finished();
        if finished {
            thread.join().ok();
        }
        finished
    }
}

impl Drop for CloakedWindow {
    fn drop(&mut self) {
        self.close_inner();
    }
}

fn hwnd_of(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

fn foreground_handle() -> isize {
    // SAFETY: GetForegroundWindow has no arguments.
    unsafe { GetForegroundWindow() }.0 as isize
}

fn show(hwnd: isize, command: SHOW_WINDOW_CMD) {
    // SAFETY: the window belongs to this process; ShowWindow only takes values.
    unsafe { ShowWindow(hwnd_of(hwnd), command) }.as_bool();
}

fn cloaked_flags(hwnd: HWND) -> windows::core::Result<u32> {
    let mut flags = 0u32;
    // SAFETY: `flags` is a live u32 and the size passed is its size.
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            std::ptr::from_mut(&mut flags).cast(),
            size_of::<u32>() as u32,
        )
    }?;
    Ok(flags)
}

fn cloak(hwnd: HWND) -> Result<(), CloakError> {
    let on = BOOL(1);
    // SAFETY: `on` is a live BOOL and the size passed is its size.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            std::ptr::from_ref(&on).cast(),
            size_of::<BOOL>() as u32,
        )
    }?;
    if cloaked_flags(hwnd)? == 0 {
        return Err(CloakError::CloakNotApplied);
    }
    Ok(())
}

fn run(
    width: i32,
    height: i32,
    reveal: Reveal,
    foreground: isize,
    ready: &Sender<Result<isize, CloakError>>,
) {
    let hwnd = match create(width, height) {
        Ok(hwnd) => hwnd,
        Err(error) => {
            ready.send(Err(error)).ok();
            return;
        }
    };
    if let Err(error) = prepare(hwnd, reveal, foreground) {
        // SAFETY: the window was created on this thread.
        unsafe { DestroyWindow(hwnd) }.ok();
        ready.send(Err(error)).ok();
        return;
    }
    ready.send(Ok(hwnd.0 as isize)).ok();
    pump(hwnd);
}

fn prepare(hwnd: HWND, reveal: Reveal, foreground: isize) -> Result<(), CloakError> {
    cloak(hwnd)?;
    if reveal == Reveal::ShownCloaked {
        // SAFETY: the window was created on this thread and is cloaked.
        unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) }.as_bool();
        if foreground_handle() != foreground {
            return Err(CloakError::ForegroundChanged);
        }
    }
    Ok(())
}

fn create(width: i32, height: i32) -> Result<HWND, CloakError> {
    // SAFETY: a null module name asks for this executable's handle.
    let module = unsafe { GetModuleHandleW(None) }?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: HINSTANCE(module.0),
        lpszClassName: w!("GreyscreenCloakedTestWindow"),
        ..WNDCLASSW::default()
    };
    // SAFETY: the class struct and its strings outlive the call; a repeat registration just fails.
    unsafe { RegisterClassW(&class) };
    // SAFETY: the class is registered and every pointer argument is null or live.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class.lpszClassName,
            w!("Greyscreen cloaked test window"),
            WS_POPUP,
            START_POSITION,
            START_POSITION,
            width,
            height,
            None,
            None,
            Some(class.hInstance),
            None,
        )
    }?;
    Ok(hwnd)
}

fn pump(hwnd: HWND) {
    let mut next_tick = Instant::now();
    loop {
        let wait = next_tick.saturating_duration_since(Instant::now());
        // SAFETY: no handles are waited on; the call only waits for queue input or the timeout.
        unsafe {
            MsgWaitForMultipleObjects(
                None::<&[HANDLE]>,
                false,
                wait.as_millis() as u32,
                QS_ALLINPUT,
            );
        }
        let mut message = MSG::default();
        // SAFETY: `message` is a live out value and the queue belongs to this thread.
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            if message.message == WM_QUIT {
                return;
            }
            // SAFETY: the message came from this thread's queue.
            unsafe {
                TranslateMessage(&message).as_bool();
                DispatchMessageW(&message);
            }
        }
        let now = Instant::now();
        if now >= next_tick {
            next_tick = (next_tick + TICK_EVERY).max(now.checked_sub(TICK_EVERY).unwrap_or(now));
            COUNTER.fetch_add(1, Ordering::Relaxed);
            // SAFETY: the window belongs to this thread; a null rect invalidates the whole client area.
            unsafe { InvalidateRect(Some(hwnd), None, false) }.as_bool();
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            // SAFETY: the window procedure runs on the thread that created the window.
            unsafe { DestroyWindow(hwnd) }.ok();
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: PostQuitMessage takes a plain exit code.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: forwards the unhandled message with the arguments Windows passed in.
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn paint(hwnd: HWND) {
    let mut paint = PAINTSTRUCT::default();
    // SAFETY: `paint` is a live out value and the window belongs to this thread.
    let target = unsafe { BeginPaint(hwnd, &mut paint) };
    let mut client = RECT::default();
    // SAFETY: `client` is a live out value.
    if unsafe { GetClientRect(hwnd, &mut client) }.is_ok() {
        let counter = COUNTER.load(Ordering::Relaxed) as u16;
        draw(target, client.right, client.bottom, counter);
    }
    // SAFETY: balances the BeginPaint above.
    unsafe { EndPaint(hwnd, &paint) }.as_bool();
}

fn draw(target: HDC, width: i32, height: i32, counter: u16) {
    // SAFETY: every GDI object made here is owned by this function and released below.
    unsafe {
        let memory = CreateCompatibleDC(Some(target));
        let bitmap = CreateCompatibleBitmap(target, width, height);
        let previous = SelectObject(memory, bitmap.into());
        let (half_x, half_y) = (width / 2, height / 2);
        fill(memory, 0, 0, half_x, half_y, RED);
        fill(memory, half_x, 0, width, half_y, GREEN);
        fill(memory, 0, half_y, half_x, height, BLUE);
        fill(memory, half_x, half_y, width, height, YELLOW);
        let (top, bottom) = (
            height * BAND_FROM_PERCENT / 100,
            height * BAND_TO_PERCENT / 100,
        );
        for bit in 0..COUNTER_BITS as i32 {
            let on = counter >> (COUNTER_BITS as i32 - 1 - bit) & 1 == 1;
            let from = bit * width / COUNTER_BITS as i32;
            let to = (bit + 1) * width / COUNTER_BITS as i32;
            fill(
                memory,
                from,
                top,
                to,
                bottom,
                if on { WHITE } else { BLACK },
            );
        }
        BitBlt(target, 0, 0, width, height, Some(memory), 0, 0, SRCCOPY).ok();
        SelectObject(memory, previous);
        DeleteObject(bitmap.into()).as_bool();
        DeleteDC(memory).as_bool();
    }
}

fn fill(dc: HDC, left: i32, top: i32, right: i32, bottom: i32, rgb: [u8; 3]) {
    let colour = COLORREF(u32::from(rgb[0]) | u32::from(rgb[1]) << 8 | u32::from(rgb[2]) << 16);
    let rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    // SAFETY: the brush is made and deleted here, and `rect` is a live local.
    unsafe {
        let brush: HBRUSH = CreateSolidBrush(colour);
        FillRect(dc, &rect, brush);
        DeleteObject(brush.into()).as_bool();
    }
}
