use std::time::Instant;

use crate::gpu_usage::GpuUsageSampler;
use greyscreen_core::health::CaptureStats;
use windows::Win32::Foundation::{FILETIME, HWND};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::Win32::UI::WindowsAndMessaging::{GA_ROOTOWNER, GetAncestor, GetForegroundWindow};

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
