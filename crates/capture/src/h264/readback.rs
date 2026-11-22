use std::time::{Duration, Instant};

use windows::Win32::Foundation::E_POINTER;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_MAP_FLAG_DO_NOT_WAIT, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::DXGI_ERROR_WAS_STILL_DRAWING;
use windows::Win32::Media::MediaFoundation::{IMFSample, MFCreateMemoryBuffer, MFCreateSample};

use super::Result;
use crate::d3d::GpuDevice;

#[cfg(test)]
mod tests;

const MAP_TIMEOUT: Duration = Duration::from_millis(100);
const MAP_POLL: Duration = Duration::from_micros(250);

#[derive(Debug)]
pub(super) struct Readback {
    context: ID3D11DeviceContext,
    staging: ID3D11Texture2D,
    width: usize,
    height: usize,
}

impl Readback {
    pub(super) fn new(gpu: &GpuDevice, width: u32, height: u32) -> Result<Self> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_NV12,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };
        let mut staging = None;
        // SAFETY: the device is live; `desc` and `staging` outlive the call.
        unsafe {
            gpu.device
                .CreateTexture2D(&desc, None, Some(&mut staging))?
        };
        let staging = staging.ok_or_else(|| windows::core::Error::from(E_POINTER))?;
        Ok(Self {
            context: gpu.context.clone(),
            staging,
            width: width as usize,
            height: height as usize,
        })
    }

    pub(super) fn sample(
        &self,
        texture: &ID3D11Texture2D,
        pts_hns: i64,
        dur_hns: i64,
    ) -> Result<IMFSample> {
        let bytes = self.width * self.height * 3 / 2;
        // SAFETY: MFCreateMemoryBuffer has no pointer arguments.
        let buffer = unsafe { MFCreateMemoryBuffer(bytes as u32)? };
        // SAFETY: both textures are live NV12 textures of the same size on this context's device.
        unsafe { self.context.CopyResource(&self.staging, texture) };
        let mapped = self.map()?;
        let copied = self.copy_planes(&mapped, &buffer);
        // SAFETY: balances the successful Map above.
        unsafe { self.context.Unmap(&self.staging, 0) };
        copied?;
        // SAFETY: MFCreateSample has no pointer arguments.
        let sample = unsafe { MFCreateSample()? };
        // SAFETY: the sample and buffer are live and owned by this call.
        unsafe {
            buffer.SetCurrentLength(bytes as u32)?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(pts_hns)?;
            sample.SetSampleDuration(dur_hns)?;
        }
        Ok(sample)
    }

    // A waiting Map holds the device lock, which would stall the capture callback, so it polls instead.
    fn map(&self) -> Result<D3D11_MAPPED_SUBRESOURCE> {
        let deadline = Instant::now() + MAP_TIMEOUT;
        loop {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            // SAFETY: the staging texture is CPU readable and `mapped` outlives the call.
            let result = fault!(StagingMap).and_then(|()| unsafe {
                self.context.Map(
                    &self.staging,
                    0,
                    D3D11_MAP_READ,
                    D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
                    Some(&mut mapped),
                )
            });
            match result {
                Ok(()) => return Ok(mapped),
                Err(error)
                    if error.code() == DXGI_ERROR_WAS_STILL_DRAWING
                        && Instant::now() < deadline =>
                {
                    std::thread::sleep(MAP_POLL);
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn copy_planes(
        &self,
        mapped: &D3D11_MAPPED_SUBRESOURCE,
        buffer: &windows::Win32::Media::MediaFoundation::IMFMediaBuffer,
    ) -> Result<()> {
        let pitch = mapped.RowPitch as usize;
        let rows = self.height * 3 / 2;
        let mut data = std::ptr::null_mut();
        let mut capacity = 0u32;
        // SAFETY: the out pointers refer to live locals.
        unsafe { buffer.Lock(&mut data, Some(&mut capacity), None)? };
        if data.is_null() || mapped.pData.is_null() || (capacity as usize) < rows * self.width {
            // SAFETY: balances the successful Lock above.
            unsafe { buffer.Unlock()? };
            return Err(windows::core::Error::from(E_POINTER).into());
        }
        let source = mapped.pData.cast::<u8>().cast_const();
        for row in 0..rows {
            // SAFETY: an NV12 mapping holds `rows` rows of `pitch` bytes, the chroma plane right after the luma plane; the buffer holds `rows * width` bytes.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    source.add(row * pitch),
                    data.add(row * self.width),
                    self.width,
                );
            }
        }
        // SAFETY: balances the successful Lock above.
        unsafe { buffer.Unlock()? };
        Ok(())
    }
}
