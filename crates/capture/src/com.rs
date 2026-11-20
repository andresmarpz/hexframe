use std::marker::PhantomData;

use windows::Win32::Media::MediaFoundation::{MF_VERSION, MFSTARTUP_FULL, MFShutdown, MFStartup};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

#[derive(Debug)]
pub struct ComGuard {
    _not_send: PhantomData<*const ()>,
}

impl ComGuard {
    pub fn mta() -> windows::core::Result<Self> {
        // SAFETY: no pointers are passed; the matching CoUninitialize runs on drop on the same thread.
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok()?;
        Ok(Self {
            _not_send: PhantomData,
        })
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: balances the successful CoInitializeEx in `mta` on this thread.
        unsafe { CoUninitialize() };
    }
}

#[derive(Debug)]
pub struct MfGuard {
    _private: (),
}

impl MfGuard {
    pub fn start() -> windows::core::Result<Self> {
        // SAFETY: no pointers are passed; the matching MFShutdown runs on drop.
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL)? };
        Ok(Self { _private: () })
    }
}

impl Drop for MfGuard {
    fn drop(&mut self) {
        // SAFETY: balances the successful MFStartup in `start`.
        if unsafe { MFShutdown() }.is_err() {
            tracing::warn!("MFShutdown failed");
        }
    }
}
