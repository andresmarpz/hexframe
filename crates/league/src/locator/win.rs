// Window search ported from OBS Studio libobs/util/windows/window-helpers.c
// Copyright (C) OBS Studio contributors. Licensed under the GNU General Public
// License, version 2 or (at your option) any later version.

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId,
};
use windows::core::{BOOL, PWSTR};

use super::GameLocator;
use super::rule::{Candidate, WINDOW_CLASS, is_game_window, text_from_wide};

const CLASS_BUFFER_UNITS: usize = 256;
const IMAGE_PATH_UNITS: usize = 32_768;
const SLOW_SEARCH: Duration = Duration::from_millis(50);

#[derive(Debug, Default)]
pub struct WindowLocator;

impl WindowLocator {
    pub fn new() -> Self {
        Self
    }
}

impl GameLocator for WindowLocator {
    fn find(&mut self) -> Option<(isize, u32)> {
        let started = Instant::now();
        let mut search = Search {
            own_pid: std::process::id(),
            found: None,
        };
        let param = LPARAM(&mut search as *mut Search as isize);
        // SAFETY: the callback only uses `search`, which outlives this synchronous call.
        // EnumWindows reports an error when the callback stops the walk, so the result is unused.
        unsafe { EnumWindows(Some(visit), param) }.unwrap_or(());
        let elapsed = started.elapsed();
        if search.found.is_none() && elapsed > SLOW_SEARCH {
            tracing::debug!(?elapsed, "window search without a match was slow");
        }
        search.found
    }
}

struct Search {
    own_pid: u32,
    found: Option<(isize, u32)>,
}

unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam is the `Search` that `find` passed and keeps alive for the whole walk.
    let search = unsafe { &mut *(lparam.0 as *mut Search) };
    match inspect(hwnd, search.own_pid) {
        Some(pid) => {
            search.found = Some((hwnd.0 as isize, pid));
            BOOL(0)
        }
        None => BOOL(1),
    }
}

fn inspect(hwnd: HWND, own_pid: u32) -> Option<u32> {
    let class = class_name(hwnd)?;
    if class != WINDOW_CLASS {
        return None;
    }
    let mut pid = 0u32;
    // SAFETY: `pid` is a live local that receives the process id.
    let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if thread == 0 || pid == 0 || pid == own_pid {
        return None;
    }
    let image_path = image_path(pid)?;
    let candidate = Candidate {
        class: &class,
        image_path: &image_path,
        pid,
    };
    is_game_window(&candidate, own_pid).then_some(pid)
}

fn class_name(hwnd: HWND) -> Option<String> {
    let mut buffer = [0u16; CLASS_BUFFER_UNITS];
    // SAFETY: the slice is a live buffer; the call writes at most its length.
    let written = unsafe { GetClassNameW(hwnd, &mut buffer) };
    let written = usize::try_from(written).ok().filter(|&n| n > 0)?;
    Some(text_from_wide(&buffer, written))
}

fn image_path(pid: u32) -> Option<String> {
    // SAFETY: OpenProcess takes plain values; the returned handle is closed below.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buffer = vec![0u16; IMAGE_PATH_UNITS];
    let mut length = buffer.len() as u32;
    // SAFETY: `buffer` holds `length` UTF-16 units and the handle is open.
    let queried = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    // SAFETY: the handle came from OpenProcess above and is not used afterwards.
    if unsafe { CloseHandle(process) }.is_err() {
        tracing::debug!("CloseHandle failed for game process handle");
    }
    queried.ok()?;
    Some(text_from_wide(&buffer, length as usize))
}
