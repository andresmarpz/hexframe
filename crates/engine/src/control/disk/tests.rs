use std::path::Path;
use std::time::Duration;

use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::disk::BYTES_PER_GB;

use super::*;
use crate::storage::ActiveFolder;

struct Fixed(Option<u64>);

impl FreeSpace for Fixed {
    fn free_bytes(&self, _: &Path) -> Option<u64> {
        self.0
    }
}

fn received(rx: &Receiver<Internal>) -> DiskCheck {
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Internal::DiskFree(check)) => check,
        _ => panic!("no disk check result"),
    }
}

#[test]
fn a_check_that_panics_still_reports_no_reading() {
    let (tx, rx) = unbounded();
    let reply = ReplyOnDrop(Some(tx));
    let worker = std::thread::spawn(move || {
        let _reply = reply;
        panic!("free space call failed");
    });
    assert!(worker.join().is_err());
    let check = received(&rx);
    assert_eq!(check.free, None);
    assert!(check.cleaned.is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn a_check_that_sends_does_not_report_twice() {
    let (tx, rx) = unbounded();
    ReplyOnDrop(Some(tx)).send(DiskCheck {
        free: Some(7),
        cleaned: None,
    });
    assert_eq!(received(&rx).free, Some(7));
    assert!(rx.try_recv().is_err());
}

#[test]
fn a_roomy_disk_cleans_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let active = ActiveFolder::default();
    let check = run_check(&Fixed(Some(10 * BYTES_PER_GB)), dir.path(), true, &active);
    assert_eq!(check.free, Some(10 * BYTES_PER_GB));
    assert!(check.cleaned.is_none());
}

#[test]
fn a_full_disk_without_a_storage_limit_is_never_cleaned() {
    let dir = tempfile::tempdir().unwrap();
    let active = ActiveFolder::default();
    let check = run_check(&Fixed(Some(1)), dir.path(), false, &active);
    assert_eq!(check.free, Some(1));
    assert!(check.cleaned.is_none());
}

#[test]
fn an_unreadable_disk_gives_no_reading() {
    let dir = tempfile::tempdir().unwrap();
    let check = run_check(&Fixed(None), dir.path(), true, &ActiveFolder::default());
    assert_eq!(check.free, None);
}
