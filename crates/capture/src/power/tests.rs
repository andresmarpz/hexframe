use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::core::{Error, HRESULT, Result};

use super::{PowerApi, PowerGuard};

#[derive(Clone, Default)]
struct Fake {
    calls: Rc<RefCell<Vec<bool>>>,
    fail: bool,
}

impl PowerApi for Fake {
    fn set(&mut self, on: bool) -> Result<()> {
        if self.fail {
            return Err(Error::from_hresult(HRESULT(-1)));
        }
        self.calls.borrow_mut().push(on);
        Ok(())
    }
}

#[test]
fn calls_the_os_once_per_state_change() {
    let fake = Fake::default();
    let mut guard = PowerGuard::new(fake.clone());
    guard.hold();
    guard.hold();
    assert!(guard.is_held());
    guard.release();
    guard.release();
    assert!(!guard.is_held());
    drop(guard);
    assert_eq!(*fake.calls.borrow(), vec![true, false]);
}

#[test]
fn drop_releases_a_held_request() {
    let fake = Fake::default();
    let mut guard = PowerGuard::new(fake.clone());
    guard.hold();
    drop(guard);
    assert_eq!(*fake.calls.borrow(), vec![true, false]);
}

#[test]
fn drop_without_hold_does_not_call_the_os() {
    let fake = Fake::default();
    drop(PowerGuard::new(fake.clone()));
    assert!(fake.calls.borrow().is_empty());
}

#[test]
fn a_panicking_stop_still_releases() {
    let fake = Fake::default();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut guard = PowerGuard::new(fake.clone());
        guard.hold();
        panic!("stop failed");
    }));
    assert!(result.is_err());
    assert_eq!(*fake.calls.borrow(), vec![true, false]);
}

#[test]
fn a_failed_hold_is_not_recorded_as_held() {
    let mut guard = PowerGuard::new(Fake {
        fail: true,
        ..Fake::default()
    });
    guard.hold();
    assert!(!guard.is_held());
}
