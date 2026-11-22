use std::time::{Duration, Instant};

use windows::Win32::Foundation::{E_FAIL, E_OUTOFMEMORY};

use super::{Armed, Point, hit};

#[test]
fn nothing_armed_changes_nothing() {
    let _armed = Armed::new();
    assert!(hit(Point::Mmcss).is_ok());
    assert!(hit(Point::StagingMap).is_ok());
}

#[test]
fn an_armed_error_is_returned_only_by_its_point() {
    let _armed = Armed::new().fail(Point::Mmcss, E_OUTOFMEMORY);
    assert_eq!(hit(Point::Mmcss).unwrap_err().code(), E_OUTOFMEMORY);
    assert!(hit(Point::GpuPriority).is_ok());
}

#[test]
fn an_armed_stall_makes_the_call_take_that_long() {
    let _armed = Armed::new().stall(Point::GpuCopyWait, Duration::from_millis(80));
    let started = Instant::now();
    assert!(hit(Point::GpuCopyWait).is_ok());
    assert!(started.elapsed() >= Duration::from_millis(80));
}

#[test]
fn dropping_the_guard_disarms_every_point() {
    drop(Armed::new().fail(Point::Mmcss, E_FAIL));
    let _armed = Armed::new();
    assert!(hit(Point::Mmcss).is_ok());
}

#[test]
fn an_armed_adapter_fault_makes_the_warp_device_fail() {
    let _armed = Armed::new().fail(Point::AdapterOpen, E_FAIL);
    let error = crate::testing::warp::device().unwrap_err();
    assert_eq!(error.code(), E_FAIL);
}

#[test]
fn the_warp_device_builds_with_nothing_armed() {
    let _armed = Armed::new();
    let gpu = crate::testing::warp::device().unwrap();
    assert!(!gpu.adapter_name.is_empty());
}
