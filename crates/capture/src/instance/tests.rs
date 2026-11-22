use windows::Win32::Foundation::E_ACCESSDENIED;

use super::{Claim, claim};
use crate::fault::{Armed, Point};

#[test]
fn a_failed_event_creation_starts_no_waiter_thread() {
    let _armed = Armed::new().fail(Point::SingleInstanceEvent, E_ACCESSDENIED);
    let Claim::First(signal) = claim("greyscreen-test-no-waiter") else {
        panic!("a failed creation must not report another instance");
    };
    assert_eq!(signal.event, 0);
    assert_eq!(signal.quit_event, 0);
}
