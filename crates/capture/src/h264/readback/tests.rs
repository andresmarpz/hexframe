use windows::Win32::Foundation::E_FAIL;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_SHADER_RESOURCE, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC};
use windows::Win32::Media::MediaFoundation::MFCreateMemoryBuffer;

use super::Readback;
use crate::com::MfGuard;
use crate::d3d::GpuDevice;
use crate::fault::{Armed, Point};
use crate::testing::warp;

const HNS: i64 = 166_667;

fn nv12_texture(gpu: &GpuDevice, width: u32, height: u32) -> ID3D11Texture2D {
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
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let mut texture = None;
    // SAFETY: the device is live; `desc` and `texture` outlive the call.
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut texture))
            .unwrap()
    };
    texture.unwrap()
}

#[test]
fn repeated_samples_leave_the_staging_texture_mappable() {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let gpu = warp::device().unwrap();
    let texture = nv12_texture(&gpu, 64, 64);
    let readback = Readback::new(&gpu, 64, 64).unwrap();
    for _ in 0..5 {
        readback.sample(&texture, 0, HNS).unwrap();
    }
}

#[test]
fn a_failed_map_returns_an_error_and_the_next_sample_works() {
    let (_mf, gpu, texture, readback) = {
        let _armed = Armed::new();
        let mf = MfGuard::start().unwrap();
        let gpu = warp::device().unwrap();
        let texture = nv12_texture(&gpu, 64, 64);
        let readback = Readback::new(&gpu, 64, 64).unwrap();
        (mf, gpu, texture, readback)
    };
    {
        let _armed = Armed::new().fail(Point::StagingMap, E_FAIL);
        assert!(readback.sample(&texture, 0, HNS).is_err());
    }
    let _armed = Armed::new();
    assert!(readback.sample(&texture, 0, HNS).is_ok());
}

#[test]
fn a_row_pitch_above_the_width_keeps_every_row_of_both_planes() {
    let _armed = Armed::new();
    let _mf = MfGuard::start().unwrap();
    let gpu = warp::device().unwrap();
    let (width, height, pitch) = (16usize, 8usize, 64usize);
    let readback = Readback::new(&gpu, width as u32, height as u32).unwrap();
    let rows = height * 3 / 2;
    let mut source = vec![0xEEu8; rows * pitch];
    for row in 0..rows {
        for x in 0..width {
            source[row * pitch + x] = (row * 16 + x) as u8;
        }
    }
    let mapped = D3D11_MAPPED_SUBRESOURCE {
        pData: source.as_mut_ptr().cast(),
        RowPitch: pitch as u32,
        DepthPitch: (rows * pitch) as u32,
    };
    // SAFETY: MFCreateMemoryBuffer has no pointer arguments.
    let buffer = unsafe { MFCreateMemoryBuffer((width * rows) as u32).unwrap() };
    readback.copy_planes(&mapped, &buffer).unwrap();
    let mut data = std::ptr::null_mut();
    // SAFETY: the out pointer refers to a live local.
    unsafe { buffer.Lock(&mut data, None, None).unwrap() };
    // SAFETY: the buffer holds `width * rows` bytes and stays locked while the slice lives.
    let packed = unsafe { std::slice::from_raw_parts(data, width * rows) }.to_vec();
    // SAFETY: balances the successful Lock above.
    unsafe { buffer.Unlock().unwrap() };
    for row in 0..rows {
        for x in 0..width {
            assert_eq!(packed[row * width + x], (row * 16 + x) as u8, "row {row}");
        }
    }
}

#[test]
fn a_readback_the_device_cannot_create_is_an_error_not_a_panic() {
    let gpu = warp::device().unwrap();
    assert!(Readback::new(&gpu, 0, 0).is_err());
}
