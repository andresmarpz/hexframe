#![cfg(windows)]
#![allow(clippy::expect_used)]

use std::process::Command;

#[test]
fn a_fault_in_a_worker_thread_leaves_a_report_and_a_dump() {
    let dir = std::env::temp_dir().join(format!("greyscreen-crash-cli-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let status = Command::new(env!("CARGO_BIN_EXE_greyscreen-cli"))
        .args(["crash-test", "--dir"])
        .arg(&dir)
        .status()
        .expect("run cli");
    assert!(!status.success());

    let report = std::fs::read_to_string(dir.join("crash.txt")).expect("crash report");
    assert!(report.contains("code=0xc0000005"), "{report}");
    assert!(report.contains("greyscreen-cli.exe+0x"), "{report}");
    assert!(report.contains("app="), "{report}");
    assert!(report.contains("os=10.0."), "{report}");
    let frames = report.matches("crash: stack #").count();
    assert!(frames >= 8, "{frames} frames\n{report}");

    let dump = std::fs::read_dir(&dir)
        .expect("crash dir")
        .flatten()
        .find(|e| e.file_name().to_string_lossy().ends_with(".dmp"))
        .expect("dump file");
    let bytes = std::fs::read(dump.path()).expect("read dump");
    assert_eq!(&bytes[..4], b"MDMP");
    std::fs::remove_dir_all(&dir).ok();
}
