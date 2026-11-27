use std::marker::PhantomData;
use std::sync::OnceLock;

use windows::Win32::Media::MediaFoundation::{MF_VERSION, MFSTARTUP_FULL, MFShutdown, MFStartup};
use windows::Win32::System::Com::{
    COINIT_MULTITHREADED, CoIncrementMTAUsage, CoInitializeEx, CoUninitialize,
};

#[derive(Debug)]
pub struct ComGuard {
    _not_send: PhantomData<*const ()>,
}

impl ComGuard {
    pub fn mta() -> windows::core::Result<Self> {
        pin_process_mta();
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
        pin_process_media_foundation();
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

// windows-rs caches WinRT factories process-wide, so COM and Media Foundation must never fully shut down.
fn pin_process_mta() {
    static PIN: OnceLock<()> = OnceLock::new();
    PIN.get_or_init(|| {
        // SAFETY: no pointers are passed; the cookie is never released so the MTA lives for the process.
        match unsafe { CoIncrementMTAUsage() } {
            Ok(_cookie) => {}
            Err(error) => tracing::warn!(%error, "cannot keep the COM MTA alive"),
        }
    });
}

fn pin_process_media_foundation() {
    static PIN: OnceLock<()> = OnceLock::new();
    PIN.get_or_init(|| {
        // SAFETY: no pointers are passed; this startup is never balanced so Media Foundation stays loaded.
        if let Err(error) = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) } {
            tracing::warn!(%error, "cannot keep Media Foundation loaded");
        }
    });
}
