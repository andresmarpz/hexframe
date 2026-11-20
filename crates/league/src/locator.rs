pub trait GameLocator: Send {
    fn find(&mut self) -> Option<(isize, u32)>;
}

#[cfg(windows)]
pub use windows_impl::WindowLocator;

#[cfg(windows)]
mod windows_impl {
    use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};
    use windows::core::{PWSTR, w};

    use super::GameLocator;

    const IMAGE_NAME: &str = "League of Legends.exe";

    #[derive(Debug, Default)]
    pub struct WindowLocator;

    impl WindowLocator {
        pub fn new() -> Self {
            Self
        }
    }

    impl GameLocator for WindowLocator {
        fn find(&mut self) -> Option<(isize, u32)> {
            // SAFETY: both arguments are valid wide-string literals or None.
            let hwnd = unsafe { FindWindowW(w!("RiotWindowClass"), None) }.ok()?;
            let mut pid = 0u32;
            // SAFETY: `pid` is a live local that receives the process id.
            let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            if thread == 0 || pid == 0 {
                return None;
            }
            image_name_matches(pid).then_some((hwnd_value(hwnd), pid))
        }
    }

    fn hwnd_value(hwnd: HWND) -> isize {
        hwnd.0 as isize
    }

    fn image_name_matches(pid: u32) -> bool {
        // SAFETY: OpenProcess takes plain values; the returned handle is closed below.
        let Ok(process) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
        else {
            return false;
        };
        let mut buffer = [0u16; MAX_PATH as usize * 4];
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
        queried.is_ok()
            && String::from_utf16_lossy(&buffer[..length as usize])
                .to_ascii_lowercase()
                .ends_with(&IMAGE_NAME.to_ascii_lowercase())
    }
}
