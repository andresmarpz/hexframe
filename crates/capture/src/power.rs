use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Power::{
    PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired, PowerRequestSystemRequired,
    PowerSetRequest,
};
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::core::{PWSTR, Result};

const POWER_REQUEST_CONTEXT_VERSION: u32 = 0;

pub(crate) trait PowerApi {
    fn set(&mut self, on: bool) -> Result<()>;
}

pub(crate) struct PowerGuard<A: PowerApi> {
    api: A,
    held: bool,
}

pub(crate) type SystemPowerGuard = PowerGuard<WindowsPower>;

impl<A: PowerApi> PowerGuard<A> {
    pub(crate) fn new(api: A) -> Self {
        Self { api, held: false }
    }

    pub(crate) fn hold(&mut self) {
        self.apply(true);
    }

    pub(crate) fn release(&mut self) {
        self.apply(false);
    }

    #[cfg(test)]
    pub(crate) fn is_held(&self) -> bool {
        self.held
    }

    fn apply(&mut self, on: bool) {
        if self.held == on {
            return;
        }
        match self.api.set(on) {
            Ok(()) => {
                self.held = on;
                tracing::info!(
                    keep_awake = on,
                    "power request {}",
                    if on { "taken" } else { "released" }
                );
            }
            Err(error) => tracing::warn!(%error, keep_awake = on, "power request change failed"),
        }
    }
}

impl<A: PowerApi> Drop for PowerGuard<A> {
    fn drop(&mut self) {
        self.release();
    }
}

impl SystemPowerGuard {
    pub(crate) fn system() -> Self {
        Self::new(WindowsPower::default())
    }
}

#[derive(Default)]
pub(crate) struct WindowsPower {
    handle: Option<HANDLE>,
}

impl WindowsPower {
    fn handle(&mut self) -> Result<HANDLE> {
        if let Some(handle) = self.handle {
            return Ok(handle);
        }
        let mut reason: Vec<u16> = "Greyscreen is recording gameplay\0".encode_utf16().collect();
        let context = REASON_CONTEXT {
            Version: POWER_REQUEST_CONTEXT_VERSION,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: PWSTR(reason.as_mut_ptr()),
            },
        };
        // SAFETY: `context` points to a null-terminated string in `reason`, which outlives the call.
        let handle = unsafe { PowerCreateRequest(&context) }?;
        self.handle = Some(handle);
        Ok(handle)
    }
}

impl PowerApi for WindowsPower {
    fn set(&mut self, on: bool) -> Result<()> {
        let handle = self.handle()?;
        for kind in [PowerRequestSystemRequired, PowerRequestDisplayRequired] {
            // SAFETY: `handle` is a live power request handle owned by this value.
            let result = unsafe {
                if on {
                    PowerSetRequest(handle, kind)
                } else {
                    PowerClearRequest(handle, kind)
                }
            };
            if on && let Err(error) = result {
                // SAFETY: as above.
                unsafe {
                    PowerClearRequest(handle, PowerRequestSystemRequired).ok();
                    PowerClearRequest(handle, PowerRequestDisplayRequired).ok();
                }
                return Err(error);
            }
        }
        Ok(())
    }
}

impl Drop for WindowsPower {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // SAFETY: the handle came from `PowerCreateRequest` and is closed once.
            unsafe { CloseHandle(handle) }.ok();
        }
    }
}
