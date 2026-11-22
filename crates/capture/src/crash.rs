use std::alloc::{GlobalAlloc, Layout};
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use windows::Wdk::System::SystemServices::RtlGetVersion;
use windows::Win32::Foundation::{HANDLE, HMODULE};
use windows::Win32::System::Diagnostics::Debug::{
    EXCEPTION_POINTERS, MINIDUMP_EXCEPTION_INFORMATION, MINIDUMP_TYPE,
    MiniDumpWithIndirectlyReferencedMemory, MiniDumpWithThreadInfo, MiniDumpWithUnloadedModules,
    MiniDumpWriteDump, SetUnhandledExceptionFilter,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId,
};
use windows::core::PCWSTR;

#[cfg(test)]
mod tests;

const DUMPS_KEPT: usize = 3;
const MAX_FRAMES: usize = 48;
const OOM_LINE: &[u8] = b"crash: out of memory (an allocation failed)\n";

static REPORT: OnceLock<Option<File>> = OnceLock::new();
static DUMP_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
static BUILD_INFO: OnceLock<String> = OnceLock::new();
static IN_HANDLER: AtomicBool = AtomicBool::new(false);
static OOM_REPORTED: AtomicBool = AtomicBool::new(false);

pub fn install(report_file: Option<PathBuf>, app_version: &str) {
    BUILD_INFO
        .set(format!("app={app_version} os={}", os_version()))
        .ok();
    let dir = report_file
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    if let Some(dir) = &dir {
        std::fs::create_dir_all(dir).ok();
        prune_dumps(dir);
    }
    DUMP_DIR.set(dir).ok();
    let file = report_file.and_then(|path| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
    });
    REPORT.set(file).ok();
    // SAFETY: the filter is a plain function that lives for the whole process.
    unsafe { SetUnhandledExceptionFilter(Some(on_crash)) };
}

pub struct ReportOutOfMemory<A>(pub A);

// SAFETY: every call forwards unchanged to the inner allocator, which upholds the contract.
unsafe impl<A: GlobalAlloc> GlobalAlloc for ReportOutOfMemory<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds the GlobalAlloc contract for this layout.
        let pointer = unsafe { self.0.alloc(layout) };
        if pointer.is_null() {
            report_out_of_memory();
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the caller passes a pointer this allocator returned for this layout.
        unsafe { self.0.dealloc(pointer, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds the GlobalAlloc contract for this layout.
        let pointer = unsafe { self.0.alloc_zeroed(layout) };
        if pointer.is_null() {
            report_out_of_memory();
        }
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller passes a pointer this allocator returned for this layout.
        let grown = unsafe { self.0.realloc(pointer, layout, new_size) };
        if grown.is_null() {
            report_out_of_memory();
        }
        grown
    }
}

fn report_out_of_memory() {
    if OOM_REPORTED.swap(true, Ordering::SeqCst) {
        return;
    }
    write_report(OOM_LINE);
}

pub fn fault_in_worker_thread() -> ! {
    let worker = std::thread::spawn(|| descend(10));
    worker.join().ok();
    std::process::abort()
}

#[inline(never)]
fn descend(depth: u32) -> u32 {
    if depth == 0 {
        // SAFETY: the write faults on purpose so the crash filter runs.
        unsafe { std::ptr::write_volatile(std::ptr::null_mut::<u32>(), 1) };
        return 0;
    }
    std::hint::black_box(descend(depth - 1)) + 1
}

struct Text {
    bytes: [u8; 8192],
    len: usize,
}

impl Text {
    fn new() -> Self {
        Self {
            bytes: [0; 8192],
            len: 0,
        }
    }

    fn filled(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl std::fmt::Write for Text {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let room = self.bytes.len() - self.len;
        let take = text.len().min(room);
        self.bytes[self.len..self.len + take].copy_from_slice(&text.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

fn write_report(bytes: &[u8]) {
    std::io::stderr().write_all(bytes).ok();
    if let Some(Some(file)) = REPORT.get() {
        let mut file: &File = file;
        file.write_all(bytes).ok();
    }
}

unsafe extern "system" fn on_crash(info: *const EXCEPTION_POINTERS) -> i32 {
    if IN_HANDLER.swap(true, Ordering::SeqCst) {
        return 0;
    }
    let mut text = Text::new();
    describe(info, &mut text);
    write_report(text.filled());
    let mut text = Text::new();
    write_dump(info, &mut text);
    write_report(text.filled());
    0
}

fn describe(info: *const EXCEPTION_POINTERS, text: &mut Text) {
    // SAFETY: the system passes valid exception pointers to the filter.
    let record = unsafe { info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) };
    let Some(record) = record else {
        writeln!(text, "crash: no exception record").ok();
        return;
    };
    let address = record.ExceptionAddress as usize;
    // SAFETY: GetCurrentThreadId has no preconditions.
    let thread = unsafe { GetCurrentThreadId() };
    write!(
        text,
        "crash: code=0x{:08x} at ",
        record.ExceptionCode.0 as u32
    )
    .ok();
    write_location(text, address);
    write!(text, " (address 0x{address:x}, thread {thread})").ok();
    if record.NumberParameters >= 2 {
        write!(
            text,
            " access={} target=0x{:x}",
            record.ExceptionInformation[0], record.ExceptionInformation[1]
        )
        .ok();
    }
    writeln!(text).ok();
    writeln!(
        text,
        "crash: {}",
        BUILD_INFO.get().map_or("", String::as_str)
    )
    .ok();
    // SAFETY: the system passes valid exception pointers to the filter.
    if let Some(context) = unsafe { info.as_ref().map(|i| i.ContextRecord) } {
        walk_stack(context, text);
    }
}

#[cfg(target_arch = "x86_64")]
fn walk_stack(
    context: *const windows::Win32::System::Diagnostics::Debug::CONTEXT,
    text: &mut Text,
) {
    use windows::Win32::System::Diagnostics::Debug::{
        RTL_VIRTUAL_UNWIND_HANDLER_TYPE, RtlLookupFunctionEntry, RtlVirtualUnwind,
    };

    if context.is_null() {
        return;
    }
    // SAFETY: the system passes a valid context record to the filter.
    let mut context = unsafe { *context };
    for index in 0..MAX_FRAMES {
        let pc = context.Rip;
        if pc == 0 {
            break;
        }
        write!(text, "crash: stack #{index} ").ok();
        write_location(text, pc as usize);
        writeln!(text).ok();
        let mut base = 0u64;
        // SAFETY: the lookup only reads the loaded images' unwind tables.
        let entry = unsafe { RtlLookupFunctionEntry(pc, &mut base, None) };
        if entry.is_null() {
            if index != 0 {
                break;
            }
            // SAFETY: a frame without unwind data is a leaf, so its return address is at the stack pointer.
            context.Rip = unsafe { (context.Rsp as *const u64).read() };
            context.Rsp += 8;
            continue;
        }
        let mut handler_data = std::ptr::null_mut();
        let mut frame = 0u64;
        // SAFETY: the entry and base come from the lookup above and the context is a live local copy.
        unsafe {
            RtlVirtualUnwind(
                RTL_VIRTUAL_UNWIND_HANDLER_TYPE(0),
                base,
                pc,
                entry,
                &mut context,
                &mut handler_data,
                &mut frame,
                None,
            );
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn walk_stack(
    _context: *const windows::Win32::System::Diagnostics::Debug::CONTEXT,
    _text: &mut Text,
) {
}

fn write_dump(info: *const EXCEPTION_POINTERS, text: &mut Text) {
    let Some(Some(dir)) = DUMP_DIR.get() else {
        return;
    };
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let path = dir.join(format!("crash-{seconds}.dmp"));
    let Ok(file) = File::create(&path) else {
        writeln!(text, "crash: cannot create {}", path.display()).ok();
        return;
    };
    let exception = MINIDUMP_EXCEPTION_INFORMATION {
        // SAFETY: GetCurrentThreadId has no preconditions.
        ThreadId: unsafe { GetCurrentThreadId() },
        ExceptionPointers: info as *mut EXCEPTION_POINTERS,
        ClientPointers: false.into(),
    };
    let kind = MINIDUMP_TYPE(
        MiniDumpWithThreadInfo.0
            | MiniDumpWithUnloadedModules.0
            | MiniDumpWithIndirectlyReferencedMemory.0,
    );
    // SAFETY: the process handle is the current process, the file handle outlives the call and the exception pointers come from the system.
    let written = unsafe {
        MiniDumpWriteDump(
            GetCurrentProcess(),
            GetCurrentProcessId(),
            HANDLE(file.as_raw_handle()),
            kind,
            Some(&exception),
            None,
            None,
        )
    };
    match written {
        Ok(()) => writeln!(text, "crash: dump={}", path.display()).ok(),
        Err(err) => writeln!(text, "crash: dump failed ({err})").ok(),
    };
}

fn prune_dumps(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dumps: Vec<_> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("crash-") && name.ends_with(".dmp")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    dumps.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, path) in dumps.into_iter().skip(DUMPS_KEPT) {
        std::fs::remove_file(path).ok();
    }
}

fn os_version() -> String {
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    // SAFETY: the struct is a live local with its size field set.
    let status = unsafe { RtlGetVersion(&mut info) };
    if status.0 < 0 {
        return "unknown".into();
    }
    format!(
        "{}.{}.{} arch={}",
        info.dwMajorVersion,
        info.dwMinorVersion,
        info.dwBuildNumber,
        std::env::consts::ARCH
    )
}

fn write_location(text: &mut Text, address: usize) {
    let mut module = HMODULE::default();
    let flags =
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    // SAFETY: with FROM_ADDRESS the name argument is read as an address inside a module.
    let found = unsafe { GetModuleHandleExW(flags, PCWSTR(address as *const u16), &mut module) };
    if found.is_err() {
        write!(text, "unknown+0x{address:x}").ok();
        return;
    }
    let mut name = [0u16; 260];
    // SAFETY: the buffer is a live local and the module handle was just returned.
    let len = unsafe { GetModuleFileNameW(Some(module), &mut name) } as usize;
    let name = &name[..len.min(name.len())];
    let file_start = name
        .iter()
        .rposition(|&c| c == u16::from(b'\\'))
        .map_or(0, |i| i + 1);
    for letter in char::decode_utf16(name[file_start..].iter().copied()) {
        text.write_char(letter.unwrap_or('?')).ok();
    }
    write!(text, "+0x{:x}", address.wrapping_sub(module.0 as usize)).ok();
}
