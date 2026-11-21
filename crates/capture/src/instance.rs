use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForSingleObject};
use windows::core::HSTRING;

#[derive(Debug)]
pub struct InstanceSignal {
    event: isize,
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
            return Claim::First(InstanceSignal { event: 0 });
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
    Claim::First(InstanceSignal {
        event: event.0 as isize,
    })
}

impl InstanceSignal {
    pub fn on_signal(self, show: impl Fn() + Send + 'static) {
        if self.event == 0 {
            return;
        }
        let event = self.event;
        std::thread::Builder::new()
            .name("greyscreen-instance".into())
            .spawn(move || {
                let handle = HANDLE(event as *mut core::ffi::c_void);
                // SAFETY: the handle stays open for the process; this thread only waits on it.
                while unsafe { WaitForSingleObject(handle, INFINITE) }.0 == 0 {
                    show();
                }
            })
            .ok();
    }
}
