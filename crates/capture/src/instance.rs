use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::{
    CreateEventW, EVENT_MODIFY_STATE, INFINITE, OpenEventW, SetEvent, WaitForSingleObject,
};
use windows::core::HSTRING;

#[derive(Debug)]
pub struct InstanceSignal {
    event: isize,
    quit_event: isize,
}

#[derive(Debug)]
pub enum Claim {
    First(InstanceSignal),
    AlreadyRunning,
}

pub fn claim(name: &str) -> Claim {
    let name = HSTRING::from(format!("Local\\{name}.show"));
    // SAFETY: the name outlives the call and no security attributes are passed.
    let event = match unsafe { CreateEventW(None, false, false, &name) } {
        Ok(event) => event,
        Err(error) => {
            tracing::warn!(%error, "cannot create the single-instance event");
            return Claim::First(InstanceSignal {
                event: 0,
                quit_event: 0,
            });
        }
    };
    // SAFETY: reads the calling thread's last error right after CreateEventW.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: the event handle was just returned and is closed once.
        unsafe {
            SetEvent(event).ok();
            CloseHandle(event).ok();
        }
        return Claim::AlreadyRunning;
    }
    let quit_name = HSTRING::from(format!("Local\\{name}.quit"));
    // SAFETY: the name outlives the call and no security attributes are passed.
    let quit_event = match unsafe { CreateEventW(None, false, false, &quit_name) } {
        Ok(handle) => handle.0 as isize,
        Err(error) => {
            tracing::warn!(%error, "cannot create the quit event");
            0
        }
    };
    Claim::First(InstanceSignal {
        event: event.0 as isize,
        quit_event,
    })
}

fn open_quit_event(id: &str) -> Option<HANDLE> {
    let name = HSTRING::from(format!("Local\\{id}.quit"));
    // SAFETY: the name outlives the call; a failed open means no instance runs.
    unsafe { OpenEventW(EVENT_MODIFY_STATE, false, &name) }.ok()
}

pub fn is_running(id: &str) -> bool {
    let Some(event) = open_quit_event(id) else {
        return false;
    };
    // SAFETY: the handle was just opened and is closed once.
    unsafe { CloseHandle(event) }.ok();
    true
}

pub fn request_quit(id: &str) -> bool {
    let Some(event) = open_quit_event(id) else {
        return false;
    };
    // SAFETY: the handle was just opened and is closed once.
    unsafe {
        let signalled = SetEvent(event).is_ok();
        CloseHandle(event).ok();
        signalled
    }
}

fn spawn_waiter(name: &str, event: isize, action: impl Fn() + Send + 'static) {
    if event == 0 {
        return;
    }
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            let handle = HANDLE(event as *mut core::ffi::c_void);
            // SAFETY: the handle stays open for the process; this thread only waits on it.
            while unsafe { WaitForSingleObject(handle, INFINITE) }.0 == 0 {
                action();
            }
        })
        .ok();
}

impl InstanceSignal {
    pub fn on_signal(self, show: impl Fn() + Send + 'static, quit: impl Fn() + Send + 'static) {
        spawn_waiter("greyscreen-instance", self.event, show);
        spawn_waiter("greyscreen-quit", self.quit_event, quit);
    }
}
