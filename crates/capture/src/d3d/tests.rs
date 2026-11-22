use windows::Win32::Foundation::E_FAIL;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;

use super::GpuDevice;
use crate::fault::{Armed, Point};
use crate::frame_copy::FrameCopier;
use crate::testing::warp;
use crate::vproc::VideoConverter;

#[test]
fn a_device_limited_to_feature_level_11_0_still_builds_the_capture_objects() {
    let _armed = Armed::new();
    let adapter = warp::adapter().unwrap();
    let gpu = GpuDevice::from_adapter_with_levels(&adapter, &[D3D_FEATURE_LEVEL_11_0]).unwrap();
    // SAFETY: the device is live and the call only reads its state.
    assert_eq!(
        unsafe { gpu.device.GetFeatureLevel() },
        D3D_FEATURE_LEVEL_11_0
    );
    VideoConverter::new(&gpu, 64, 64, 30).unwrap();
    FrameCopier::new(&gpu).unwrap();
}

#[test]
fn a_failed_device_creation_does_not_stop_the_next_one() {
    {
        let _armed = Armed::new().fail(Point::AdapterOpen, E_FAIL);
        let adapter = warp::adapter().unwrap();
        let error = GpuDevice::from_adapter(&adapter).unwrap_err();
        assert_eq!(error.code(), E_FAIL);
    }
    let _armed = Armed::new();
    assert!(warp::device().is_ok());
}
