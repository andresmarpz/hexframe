use std::cell::RefCell;

use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};
use windows::Win32::Graphics::Direct3D11::D3D11_BIND_VIDEO_ENCODER;

use super::{POOL_SIZE, VideoConverter, VprocError, pool_with};
use crate::fault::{Armed, Point};
use crate::testing::warp;

const ENCODER_BIND: u32 = D3D11_BIND_VIDEO_ENCODER.0 as u32;

#[test]
fn a_failed_converter_setup_is_a_typed_error_and_the_next_setup_works() {
    {
        let _armed = Armed::new().fail(Point::ConverterSetup, E_FAIL);
        let gpu = warp::device().unwrap();
        match VideoConverter::new(&gpu, 64, 64, 30) {
            Err(VprocError::Windows(error)) => assert_eq!(error.code(), E_FAIL),
            Err(other) => panic!("wrong error: {other}"),
            Ok(_) => panic!("setup should fail"),
        }
    }
    let _armed = Armed::new();
    let gpu = warp::device().unwrap();
    assert!(VideoConverter::new(&gpu, 64, 64, 30).is_ok());
}

#[test]
fn a_refused_encoder_bind_flag_falls_back_to_a_render_target_pool() {
    let flags_seen = RefCell::new(Vec::new());
    let pool = pool_with(|flags| {
        flags_seen.borrow_mut().push(flags);
        if flags & ENCODER_BIND != 0 {
            Err(E_INVALIDARG.into())
        } else {
            Ok(flags)
        }
    })
    .unwrap();
    assert_eq!(pool.len(), POOL_SIZE);
    assert!(pool.iter().all(|flags| flags & ENCODER_BIND == 0));
    let seen = flags_seen.into_inner();
    assert_ne!(seen[0] & ENCODER_BIND, 0);
    assert!(seen[1..].iter().all(|flags| flags & ENCODER_BIND == 0));
}

#[test]
fn an_accepted_encoder_bind_flag_is_kept_for_the_whole_pool() {
    let pool = pool_with(Ok).unwrap();
    assert_eq!(pool.len(), POOL_SIZE);
    assert!(pool.iter().all(|flags| flags & ENCODER_BIND != 0));
}

#[test]
fn a_pool_that_cannot_be_built_either_way_is_an_error_after_two_tries() {
    let calls = RefCell::new(0);
    let result = pool_with::<u32>(|_| {
        *calls.borrow_mut() += 1;
        Err(E_FAIL.into())
    });
    assert_eq!(result.unwrap_err().code(), E_FAIL);
    assert_eq!(calls.into_inner(), 2);
}

#[test]
fn a_texture_failing_mid_pool_fails_the_whole_pool() {
    let made = RefCell::new(0);
    let result = pool_with(|flags| {
        let mut made = made.borrow_mut();
        *made += 1;
        if *made == 3 {
            Err(E_FAIL.into())
        } else {
            Ok(flags)
        }
    });
    assert_eq!(result.unwrap_err().code(), E_FAIL);
}

#[test]
fn an_unsupported_conversion_is_the_unsupported_error() {
    let _armed = Armed::new().fail(Point::ConversionSupport, E_FAIL);
    let gpu = warp::device().unwrap();
    assert!(matches!(
        VideoConverter::new(&gpu, 64, 64, 30),
        Err(VprocError::Unsupported)
    ));
}
