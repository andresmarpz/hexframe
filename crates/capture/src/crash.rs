use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::Diagnostics::Debug::{EXCEPTION_POINTERS, SetUnhandledExceptionFilter};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::core::PCWSTR;

static REPORT_FILE: OnceLock<Option<PathBuf>> = OnceLock::new();

pub fn install(report_file: Option<PathBuf>) {
    REPORT_FILE.set(report_file).ok();
    // SAFETY: the filter is a plain function that lives for the whole process.
    unsafe { SetUnhandledExceptionFilter(Some(on_crash)) };
}

unsafe extern "system" fn on_crash(info: *const EXCEPTION_POINTERS) -> i32 {
    let line = describe(info);
    writeln!(std::io::stderr(), "{line}").ok();
    if let Some(Some(path)) = REPORT_FILE.get()
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        writeln!(file, "{line}").ok();
    }
    0
}

fn describe(info: *const EXCEPTION_POINTERS) -> String {
    // SAFETY: the system passes valid exception pointers to the filter.
    let record = unsafe { info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) };
    let Some(record) = record else {
        return "crash: no exception record".into();
    };
    let address = record.ExceptionAddress as usize;
    let (module, base) = module_of(address);
    let detail = if record.NumberParameters >= 2 {
        format!(
            " access={} target=0x{:x}",
            record.ExceptionInformation[0], record.ExceptionInformation[1]
        )
    } else {
        String::new()
    };
    // SAFETY: GetCurrentThreadId has no preconditions.
    let thread = unsafe { GetCurrentThreadId() };
    format!(
        "crash: code=0x{:08x} at {module}+0x{:x} (address 0x{address:x}, thread {thread}){detail}",
        record.ExceptionCode.0 as u32,
        address.wrapping_sub(base),
    )
}

fn module_of(address: usize) -> (String, usize) {
    let mut module = HMODULE::default();
    let flags =
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    // SAFETY: with FROM_ADDRESS the name argument is read as an address inside a module.
    let found = unsafe { GetModuleHandleExW(flags, PCWSTR(address as *const u16), &mut module) };
    if found.is_err() {
        return ("unknown".into(), 0);
    }
    let mut name = [0u16; 260];
    // SAFETY: the buffer is a live local and the module handle was just returned.
    let len = unsafe { GetModuleFileNameW(Some(module), &mut name) } as usize;
    let path = String::from_utf16_lossy(&name[..len.min(name.len())]);
    let file = path.rsplit('\\').next().unwrap_or(&path).to_owned();
    (file, module.0 as usize)
}
