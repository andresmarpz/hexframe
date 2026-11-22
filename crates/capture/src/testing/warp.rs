use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory4};

use crate::d3d::GpuDevice;

pub fn device() -> windows::core::Result<GpuDevice> {
    // SAFETY: CreateDXGIFactory1 has no pointer arguments.
    let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory1()? };
    // SAFETY: the call writes only its return value.
    let adapter: IDXGIAdapter1 = unsafe { factory.EnumWarpAdapter()? };
    GpuDevice::from_adapter(&adapter)
}
