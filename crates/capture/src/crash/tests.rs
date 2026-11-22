#![allow(clippy::expect_used)]

use std::alloc::{GlobalAlloc, Layout};

use super::{ReportOutOfMemory, install};

struct AlwaysFails;

// SAFETY: the allocator never hands out memory, so it cannot break any allocation rule.
unsafe impl GlobalAlloc for AlwaysFails {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        std::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}

#[test]
fn a_failed_allocation_writes_one_out_of_memory_line() {
    let dir = std::env::temp_dir().join(format!("greyscreen-crash-test-{}", std::process::id()));
    let report = dir.join("crash.txt");
    install(Some(report.clone()), "test");
    let allocator = ReportOutOfMemory(AlwaysFails);
    let layout = Layout::from_size_align(64, 8).expect("layout");

    // SAFETY: the failing allocator returns null and nothing is freed.
    let first = unsafe { allocator.alloc(layout) };
    // SAFETY: as above.
    let second = unsafe { allocator.alloc(layout) };

    assert!(first.is_null() && second.is_null());
    let text = std::fs::read_to_string(&report).expect("report");
    assert_eq!(text.matches("out of memory").count(), 1, "{text}");
    std::fs::remove_dir_all(&dir).ok();
}
