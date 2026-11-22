use std::time::Instant;

use crate::d3d::{GpuDevice, luid_u64};
use crate::gpu_usage::GpuUsageSampler;
use greyscreen_core::health::CaptureStats;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HWND};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
    IDXGIAdapter3, IDXGIFactory1,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetProcessHandleCount, GetProcessTimes,
};
use windows::Win32::UI::WindowsAndMessaging::{GA_ROOTOWNER, GetAncestor, GetForegroundWindow};
use windows::core::Interface;

const BYTES_PER_MB: u64 = 1024 * 1024;

#[derive(Debug)]
pub(crate) struct ProcessSampler {
    last_wall: Instant,
    last_cpu: Option<u64>,
    target: Option<HWND>,
    gpu: GpuUsageSampler,
}

impl ProcessSampler {
    pub(crate) fn new(target: Option<isize>, adapter_luid: Option<u64>) -> Self {
        Self {
            last_wall: Instant::now(),
            last_cpu: cpu_time_hns(),
            target: target.map(|hwnd| HWND(hwnd as *mut core::ffi::c_void)),
            gpu: GpuUsageSampler::start(adapter_luid),
        }
    }

    pub(crate) fn fill_gpu(&self, stats: &mut CaptureStats) {
        let usage = self.gpu.latest();
        stats.gpu_3d_pct = usage.map(|u| u.busy_3d_pct);
        stats.gpu_3d_self_pct = usage.map(|u| u.self_3d_pct);
        stats.gpu_encode_pct = usage.map(|u| u.encode_pct);
        stats.gpu_copy_pct = usage.map(|u| u.copy_pct);
    }

    pub(crate) fn target_in_foreground(&self) -> Option<bool> {
        let target = self.target?;
        // SAFETY: both calls only read window state and accept any handle value.
        let (front, front_owner, target_owner) = unsafe {
            let front = GetForegroundWindow();
            (
                front,
                GetAncestor(front, GA_ROOTOWNER),
                GetAncestor(target, GA_ROOTOWNER),
            )
        };
        if front.0.is_null() {
            return Some(false);
        }
        Some(front == target || (!target_owner.0.is_null() && front_owner == target_owner))
    }

    pub(crate) fn sample(&mut self) -> (Option<f32>, Option<u32>) {
        let wall = self.last_wall.elapsed();
        self.last_wall = Instant::now();
        let cpu = cpu_time_hns();
        let pct = match (self.last_cpu, cpu) {
            (Some(before), Some(now)) if !wall.is_zero() => {
                let busy = now.saturating_sub(before) as f64 / 10_000_000.0;
                Some((busy / wall.as_secs_f64() * 100.0) as f32)
            }
            _ => None,
        };
        self.last_cpu = cpu;
        (pct, working_set_mb())
    }
}

fn filetime_hns(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

fn cpu_time_hns() -> Option<u64> {
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: the pseudo handle is always valid and the out pointers refer to live locals.
    unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    }
    .ok()?;
    Some(filetime_hns(kernel) + filetime_hns(user))
}

fn working_set_mb() -> Option<u32> {
    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: the pseudo handle is always valid and `counters` is a live struct of the size passed.
    unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) }.ok()?;
    u32::try_from(counters.WorkingSetSize as u64 / BYTES_PER_MB).ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessUsage {
    pub private_bytes: u64,
    pub handles: u32,
    pub threads: u32,
    pub gpu_dedicated_bytes: Option<u64>,
}

#[derive(Debug)]
pub struct UsageMeter {
    adapter: Option<IDXGIAdapter3>,
}

impl UsageMeter {
    pub fn for_primary_adapter() -> Self {
        let luid = GpuDevice::for_primary_monitor()
            .ok()
            .map(|gpu| luid_u64(gpu.adapter_luid));
        Self {
            adapter: luid.and_then(adapter_by_luid),
        }
    }

    pub fn sample(&self) -> Option<ProcessUsage> {
        Some(ProcessUsage {
            private_bytes: private_bytes()?,
            handles: handle_count()?,
            threads: thread_count()?,
            gpu_dedicated_bytes: self.adapter.as_ref().and_then(gpu_dedicated_bytes),
        })
    }
}

fn adapter_by_luid(luid: u64) -> Option<IDXGIAdapter3> {
    // SAFETY: CreateDXGIFactory1 has no pointer arguments.
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    let mut index = 0;
    // SAFETY: enumeration only reads from live COM objects and writes return values.
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(index) } {
        // SAFETY: GetDesc1 only writes to the returned value.
        let found =
            unsafe { adapter.GetDesc1() }.is_ok_and(|desc| luid_u64(desc.AdapterLuid) == luid);
        if found {
            return adapter.cast().ok();
        }
        index += 1;
    }
    None
}

fn gpu_dedicated_bytes(adapter: &IDXGIAdapter3) -> Option<u64> {
    let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
    // SAFETY: `adapter` is live and `info` is a live out struct; the query covers the calling process.
    unsafe { adapter.QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info) }.ok()?;
    Some(info.CurrentUsage)
}

fn private_bytes() -> Option<u64> {
    let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    counters.cb = size;
    // SAFETY: the pseudo handle is always valid; the extended struct starts with the basic one and `size` is its full length.
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
            size,
        )
    }
    .ok()?;
    Some(counters.PrivateUsage as u64)
}

fn handle_count() -> Option<u32> {
    let mut count = 0u32;
    // SAFETY: the pseudo handle is always valid and `count` is a live out value.
    unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }.ok()?;
    Some(count)
}

fn thread_count() -> Option<u32> {
    // SAFETY: a thread snapshot takes no process id and returns an owned handle.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }.ok()?;
    // SAFETY: the call only reads the current process id.
    let own = unsafe { GetCurrentProcessId() };
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..THREADENTRY32::default()
    };
    let mut count = 0u32;
    // SAFETY: `snapshot` is open and `entry` is a live struct with `dwSize` set.
    let mut more = unsafe { Thread32First(snapshot, &mut entry) }.is_ok();
    while more {
        if entry.th32OwnerProcessID == own {
            count += 1;
        }
        // SAFETY: same as above.
        more = unsafe { Thread32Next(snapshot, &mut entry) }.is_ok();
    }
    // SAFETY: `snapshot` was opened above and is closed once.
    unsafe { CloseHandle(snapshot) }.ok()?;
    Some(count)
}
