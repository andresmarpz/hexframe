use greyscreen_core::encoder::Adapter;
use greyscreen_core::health::driver_version_text;
use greyscreen_core::media::CaptureTarget;
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_GPU_PREFERENCE, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE,
    DXGI_GPU_PREFERENCE_MINIMUM_POWER, IDXGIAdapter, IDXGIAdapter1, IDXGIDevice, IDXGIFactory1,
    IDXGIFactory6,
};
use windows::core::Interface;

use crate::d3d::{adapter_for_monitor, luid_u64, target_monitor, wide_to_string};

pub fn for_target(target: &CaptureTarget) -> Vec<Adapter> {
    match list(target) {
        Ok(adapters) => adapters,
        Err(error) => {
            tracing::warn!(%error, "cannot list graphics adapters");
            Vec::new()
        }
    }
}

fn list(target: &CaptureTarget) -> windows::core::Result<Vec<Adapter>> {
    // SAFETY: CreateDXGIFactory1 has no pointer arguments.
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
    let display = adapter_for_monitor(&factory, target_monitor(target)).and_then(|a| luid_of(&a));
    let high_performance = preferred(&factory, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE);
    let power_saving = preferred(&factory, DXGI_GPU_PREFERENCE_MINIMUM_POWER);
    let mut adapters = Vec::new();
    let mut index = 0;
    // SAFETY: enumeration only reads from live COM objects and writes return values.
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(index) } {
        index += 1;
        // SAFETY: GetDesc1 only writes to the returned value.
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        let luid = luid_u64(desc.AdapterLuid);
        if adapters.iter().any(|a: &Adapter| a.luid == luid) {
            continue;
        }
        adapters.push(Adapter {
            luid,
            name: wide_to_string(&desc.Description),
            vendor_id: desc.VendorId,
            driver_version: driver_version(&adapter, desc.VendorId),
            display: display == Some(luid),
            high_performance: high_performance == Some(luid),
            power_saving: power_saving == Some(luid),
        });
    }
    Ok(adapters)
}

fn luid_of(adapter: &IDXGIAdapter1) -> Option<u64> {
    // SAFETY: GetDesc1 only writes to the returned value.
    unsafe { adapter.GetDesc1() }
        .ok()
        .map(|desc| luid_u64(desc.AdapterLuid))
}

fn preferred(factory: &IDXGIFactory1, preference: DXGI_GPU_PREFERENCE) -> Option<u64> {
    let factory6 = factory.cast::<IDXGIFactory6>().ok()?;
    // SAFETY: the call writes only its return value.
    let adapter =
        unsafe { factory6.EnumAdapterByGpuPreference::<IDXGIAdapter1>(0, preference) }.ok()?;
    luid_of(&adapter)
}

fn driver_version(adapter: &IDXGIAdapter1, vendor_id: u32) -> String {
    let Ok(base) = adapter.cast::<IDXGIAdapter>() else {
        return "unknown".to_owned();
    };
    // SAFETY: the interface id is a valid constant and the call only reads the live adapter.
    unsafe { base.CheckInterfaceSupport(&IDXGIDevice::IID) }.map_or_else(
        |_| "unknown".to_owned(),
        |v| driver_version_text(v, vendor_id),
    )
}
