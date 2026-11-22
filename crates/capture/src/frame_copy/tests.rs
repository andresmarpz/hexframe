use std::time::Duration;

use greyscreen_core::tear::Wait;

use super::{FrameCopier, frame_desc, texture};
use crate::fault::{Armed, Point};

const SIZE: (u32, u32) = (64, 64);

#[test]
fn a_copy_finishes_with_nothing_armed() {
    let _armed = Armed::new();
    let gpu = crate::testing::warp::device().unwrap();
    let source = texture(&gpu, frame_desc(SIZE)).unwrap();
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let copied = copier.copy(&source, SIZE, 0).unwrap();
    assert!(matches!(copied.wait, Wait::Done(_)));
}

#[test]
fn an_armed_stall_on_the_copy_wait_times_the_copy_out() {
    let _armed = Armed::new().stall(Point::GpuCopyWait, Duration::from_millis(1));
    let gpu = crate::testing::warp::device().unwrap();
    let source = texture(&gpu, frame_desc(SIZE)).unwrap();
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let copied = copier.copy(&source, SIZE, 0).unwrap();
    assert!(matches!(copied.wait, Wait::TimedOut(_)));
}
