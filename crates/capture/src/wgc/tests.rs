use greyscreen_core::media::CaptureTarget;
use windows::Win32::Foundation::E_NOTIMPL;

use windows::Graphics::SizeInt32;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};

use super::{WgcCapture, WgcError, WgcOptions, Worker};
use crate::clock::QpcMap;
use crate::com::MfGuard;
use crate::fault::{Armed, Point};
use crate::frame_copy::FrameCopier;

#[test]
fn capture_starts_when_the_border_and_update_interval_calls_are_not_implemented() {
    let _armed = Armed::new()
        .fail(Point::CaptureBorder, E_NOTIMPL)
        .fail(Point::CaptureMinUpdateInterval, E_NOTIMPL);
    let _mf = MfGuard::start().unwrap();
    let gpu = crate::testing::warp::device().unwrap();
    let options = WgcOptions {
        fps: 30,
        cursor: false,
        display_hz: 60,
    };
    match WgcCapture::start(&gpu, &CaptureTarget::PrimaryMonitor, QpcMap::now(), options) {
        Ok(capture) => {
            assert!(!capture.border_hidden());
            capture.stop();
        }
        Err(WgcError::Unsupported) => {}
        Err(error) => panic!("capture must start without the optional calls: {error}"),
    }
}

fn surface(gpu: &crate::d3d::GpuDevice, width: u32, height: u32) -> ID3D11Texture2D {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
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

fn copy_size(content: (i32, i32), surface_size: (u32, u32)) -> ((u32, u32), (u32, u32)) {
    let _armed = Armed::new();
    let gpu = crate::testing::warp::device().unwrap();
    let source = surface(&gpu, surface_size.0, surface_size.1);
    let mut worker = Worker {
        copier: Some(FrameCopier::new(&gpu).unwrap()),
        thumb: Vec::new(),
        pool_size: (0, 0),
    };
    let content = SizeInt32 {
        Width: content.0,
        Height: content.1,
    };
    let taken = worker.copy(&source, content, 0).unwrap();
    let copier = worker.copier.as_ref().unwrap();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: `desc` is a live out value and the ring texture is live.
    unsafe { copier.texture(0).unwrap().GetDesc(&mut desc) };
    (taken.size, (desc.Width, desc.Height))
}

#[test]
fn a_content_size_larger_than_the_surface_is_clamped_to_the_surface() {
    let (published, ring) = copy_size((1921, 1081), (1920, 1080));
    assert_eq!(published, (1920, 1080));
    assert_eq!(ring, (1920, 1080));
}

#[test]
fn a_content_size_smaller_than_the_surface_is_published_as_the_content() {
    let (published, ring) = copy_size((1280, 720), (1920, 1080));
    assert_eq!(published, (1280, 720));
    assert_eq!(ring, (1280, 720));
}
