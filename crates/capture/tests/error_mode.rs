#![cfg(windows)]

use greyscreen_capture::probe;

const SEM_FAILCRITICALERRORS: u32 = 0x0001;

#[test]
fn process_init_sets_fail_critical_errors() {
    probe::init_process().ok();
    assert_ne!(probe::error_mode() & SEM_FAILCRITICALERRORS, 0);
}
