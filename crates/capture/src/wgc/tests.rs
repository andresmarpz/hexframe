use greyscreen_core::media::CaptureTarget;
use windows::Win32::Foundation::E_NOTIMPL;

use super::{WgcCapture, WgcError, WgcOptions};
use crate::clock::QpcMap;
use crate::com::MfGuard;
use crate::fault::{Armed, Point};

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
