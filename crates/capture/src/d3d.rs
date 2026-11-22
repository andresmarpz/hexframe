use std::sync::Once;

use greyscreen_core::health::driver_version_text;
use greyscreen_core::health::gpu::{GpuScheduling, hags_label, support_label};
use greyscreen_core::media::CaptureTarget;
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Wdk::Graphics::Direct3D::{
    D3DKMT_SCHEDULINGPRIORITYCLASS_HIGH, D3DKMTSetProcessSchedulingPriorityClass,
};
use windows::Win32::Foundation::{HMODULE, HWND, LUID};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_SDK_VERSION,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Multithread,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ERROR_NOT_FOUND, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IDXGIAdapter,
    IDXGIAdapter1, IDXGIDevice, IDXGIFactory1, IDXGIFactory6,
};
use windows::Win32::Graphics::Gdi::{
    HMONITOR, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MonitorFromWindow,
};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::core::Interface;

#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub adapter_luid: LUID,
    pub adapter_name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub subsystem_id: u32,
    pub revision: u32,
    pub driver_version: String,
    pub scheduling: Option<GpuScheduling>,
}

impl GpuDevice {
    pub fn for_target(target: &CaptureTarget) -> windows::core::Result<Self> {
        Self::for_monitor(target_monitor(target))
    }

    pub fn for_primary_monitor() -> windows::core::Result<Self> {
        Self::for_target(&CaptureTarget::PrimaryMonitor)
    }

    pub fn for_luid(luid: u64) -> windows::core::Result<Self> {
        // SAFETY: CreateDXGIFactory1 has no pointer arguments.
        let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
        let mut index = 0;
        // SAFETY: enumeration only reads from live COM objects and writes return values.
        while let Ok(adapter) = unsafe { factory.EnumAdapters1(index) } {
            // SAFETY: GetDesc1 only writes to the returned value.
            if unsafe { adapter.GetDesc1() }.is_ok_and(|desc| luid_u64(desc.AdapterLuid) == luid) {
                return Self::create(&adapter);
            }
            index += 1;
        }
        Err(DXGI_ERROR_NOT_FOUND.into())
    }

    pub fn winrt_device(&self) -> windows::core::Result<IDirect3DDevice> {
        let dxgi: IDXGIDevice = self.device.cast()?;
        // SAFETY: `dxgi` is a live DXGI device owned by this call.
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
        inspectable.cast()
    }

    #[cfg(any(test, feature = "fault-injection"))]
    pub(crate) fn from_adapter(adapter: &IDXGIAdapter1) -> windows::core::Result<Self> {
        Self::create(adapter)
    }

    fn for_monitor(monitor: HMONITOR) -> windows::core::Result<Self> {
        // SAFETY: CreateDXGIFactory1 has no pointer arguments.
        let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
        let adapter = match adapter_for_monitor(&factory, monitor) {
            Some(adapter) => adapter,
            None => fallback_adapter(&factory)?,
        };
        Self::create(&adapter)
    }

    #[cfg(any(test, feature = "fault-injection"))]
    pub(crate) fn from_adapter_with_levels(
        adapter: &IDXGIAdapter1,
        levels: &[D3D_FEATURE_LEVEL],
    ) -> windows::core::Result<Self> {
        Self::create_with_levels(adapter, levels)
    }

    fn create(adapter: &IDXGIAdapter1) -> windows::core::Result<Self> {
        Self::create_with_levels(adapter, &[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0])
    }

    fn create_with_levels(
        adapter: &IDXGIAdapter1,
        levels: &[D3D_FEATURE_LEVEL],
    ) -> windows::core::Result<Self> {
        fault!(AdapterOpen)?;
        // SAFETY: GetDesc1 only writes to the returned value.
        let desc = unsafe { adapter.GetDesc1()? };
        let adapter_name = wide_to_string(&desc.Description);
        let base: IDXGIAdapter = adapter.cast()?;
        // SAFETY: the interface id is a valid constant and the call only reads the live adapter.
        let driver_version = unsafe { base.CheckInterfaceSupport(&IDXGIDevice::IID) }.map_or_else(
            |_| "unknown".to_owned(),
            |v| driver_version_text(v, desc.VendorId),
        );
        let mut device = None;
        let mut context = None;
        // SAFETY: the out pointers refer to live locals and the adapter outlives the call.
        unsafe {
            D3D11CreateDevice(
                &base,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
                Some(levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
        }
        let (device, context) = device
            .zip(context)
            .ok_or_else(|| windows::core::Error::from(DXGI_ERROR_NOT_FOUND))?;
        let multithread: ID3D11Multithread = context.cast()?;
        // SAFETY: the interface was just obtained from the live immediate context.
        let _previous = unsafe { multithread.SetMultithreadProtected(true) };
        let scheduling = crate::hags::query(desc.AdapterLuid);
        tracing::info!(
            adapter = %adapter_name,
            hags = hags_label(scheduling),
            hags_support = support_label(scheduling),
            device_id = format_args!("{:#06x}", desc.DeviceId),
            subsystem_id = format_args!("{:#010x}", desc.SubSysId),
            revision = desc.Revision,
            "gpu scheduling"
        );
        raise_gpu_priority(&device, scheduling);
        Ok(Self {
            device,
            context,
            adapter_luid: desc.AdapterLuid,
            adapter_name,
            vendor_id: desc.VendorId,
            device_id: desc.DeviceId,
            subsystem_id: desc.SubSysId,
            revision: desc.Revision,
            driver_version,
            scheduling,
        })
    }
}

const GPU_THREAD_PRIORITY: i32 = 7;

fn raise_gpu_priority(device: &ID3D11Device, scheduling: Option<GpuScheduling>) {
    let hags = hags_label(scheduling);
    match device.cast::<IDXGIDevice>() {
        Ok(dxgi) => {
            // SAFETY: `dxgi` is a live DXGI device and the call only takes an integer.
            let result = fault!(GpuPriority)
                .and_then(|()| unsafe { dxgi.SetGPUThreadPriority(GPU_THREAD_PRIORITY) });
            match result {
                Ok(()) => {
                    tracing::info!(
                        priority = GPU_THREAD_PRIORITY,
                        hags,
                        "gpu thread priority set"
                    );
                }
                Err(error) => tracing::warn!(%error, hags, "set gpu thread priority failed"),
            }
        }
        Err(error) => tracing::warn!(%error, "gpu thread priority needs a dxgi device"),
    }
    static PROCESS_CLASS: Once = Once::new();
    PROCESS_CLASS.call_once(|| {
        // SAFETY: the pseudo handle from GetCurrentProcess is always valid and the call takes a plain enum value.
        let status = unsafe {
            D3DKMTSetProcessSchedulingPriorityClass(
                GetCurrentProcess(),
                D3DKMT_SCHEDULINGPRIORITYCLASS_HIGH,
            )
        };
        if status.is_ok() {
            tracing::info!(hags, "gpu process scheduling class set to high");
        } else {
            tracing::warn!(
                status = status.0,
                hags,
                "set gpu process scheduling class failed"
            );
        }
    });
}

pub(crate) fn target_monitor(target: &CaptureTarget) -> HMONITOR {
    match *target {
        CaptureTarget::GameWindow { hwnd, .. } => {
            let hwnd = HWND(hwnd as *mut core::ffi::c_void);
            // SAFETY: MonitorFromWindow accepts any handle value and falls back to the nearest monitor.
            unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }
        }
        // SAFETY: a null window handle with DEFAULTTOPRIMARY returns the primary monitor.
        CaptureTarget::PrimaryMonitor => unsafe {
            MonitorFromWindow(HWND::default(), MONITOR_DEFAULTTOPRIMARY)
        },
    }
}

pub(crate) fn luid_u64(luid: LUID) -> u64 {
    (u64::from(luid.HighPart as u32) << 32) | u64::from(luid.LowPart)
}

pub(crate) fn adapter_for_monitor(
    factory: &IDXGIFactory1,
    monitor: HMONITOR,
) -> Option<IDXGIAdapter1> {
    let mut adapter_index = 0;
    // SAFETY: enumeration calls only read from live COM objects and write return values.
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(adapter_index) } {
        let mut output_index = 0;
        // SAFETY: as above.
        while let Ok(output) = unsafe { adapter.EnumOutputs(output_index) } {
            // SAFETY: as above.
            if let Ok(desc) = unsafe { output.GetDesc() }
                && desc.Monitor == monitor
            {
                return Some(adapter);
            }
            output_index += 1;
        }
        adapter_index += 1;
    }
    None
}

fn fallback_adapter(factory: &IDXGIFactory1) -> windows::core::Result<IDXGIAdapter1> {
    if let Ok(factory6) = factory.cast::<IDXGIFactory6>() {
        // SAFETY: the call writes only its return value.
        let preferred = unsafe {
            factory6.EnumAdapterByGpuPreference::<IDXGIAdapter1>(
                0,
                DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE,
            )
        };
        if let Ok(adapter) = preferred {
            return Ok(adapter);
        }
    }
    // SAFETY: the call writes only its return value.
    unsafe { factory.EnumAdapters1(0) }
}

pub(crate) fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

#[cfg(test)]
mod tests;
