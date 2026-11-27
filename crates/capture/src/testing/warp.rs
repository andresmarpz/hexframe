use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory4};

use crate::d3d::GpuDevice;

// Some WARP builds refuse video support, so the tests fall back to the first hardware adapter.
pub fn device() -> windows::core::Result<GpuDevice> {
    GpuDevice::from_adapter(&adapter()?).or_else(|_| GpuDevice::from_adapter(&hardware()?))
}

pub fn adapter() -> windows::core::Result<IDXGIAdapter1> {
    // SAFETY: CreateDXGIFactory1 has no pointer arguments.
    let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory1()? };
    // SAFETY: the call writes only its return value.
    unsafe { factory.EnumWarpAdapter() }
}

fn hardware() -> windows::core::Result<IDXGIAdapter1> {
    // SAFETY: CreateDXGIFactory1 has no pointer arguments.
    let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory1()? };
    // SAFETY: the call writes only its return value.
    unsafe { factory.EnumAdapters1(0) }
}
