#![allow(unreachable_pub)]

use std::process::Command;

const OPT_OUT: &str = "GREYSCREEN_SKIP_FFMPEG_TESTS";

fn tool_available(name: &str) -> bool {
    Command::new(name).arg("-version").output().is_ok()
}

pub fn ffmpeg_ready() -> bool {
    if tool_available("ffprobe") && tool_available("ffmpeg") {
        return true;
    }
    assert!(
        std::env::var(OPT_OUT).is_ok_and(|v| v == "1"),
        "ffmpeg and ffprobe must be on PATH to run this test; install them, or set {OPT_OUT}=1 to skip the tests that need them"
    );
    eprintln!("skipped: ffprobe or ffmpeg is not on PATH and {OPT_OUT}=1");
    false
}

macro_rules! require_ffmpeg {
    () => {
        if !common::ffmpeg_ready() {
            return;
        }
    };
}
