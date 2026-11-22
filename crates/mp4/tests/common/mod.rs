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

#[allow(dead_code)]
pub mod faulty {
    use std::io::{self, Cursor, Seek, SeekFrom, Write};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use greyscreen_mp4::sink::Sink;

    pub struct FaultySink {
        inner: Arc<Mutex<Cursor<Vec<u8>>>>,
        fail_after: u64,
        kind: io::ErrorKind,
        pause: Option<Duration>,
        written: Arc<AtomicU64>,
    }

    impl FaultySink {
        pub fn new(fail_after: u64, kind: io::ErrorKind) -> Self {
            Self {
                inner: Arc::default(),
                fail_after,
                kind,
                pause: None,
                written: Arc::new(AtomicU64::new(0)),
            }
        }

        pub fn healthy() -> Self {
            Self::new(u64::MAX, io::ErrorKind::Other)
        }

        pub fn slow(pause: Duration) -> Self {
            Self {
                pause: Some(pause),
                ..Self::healthy()
            }
        }

        pub fn written(&self) -> Arc<AtomicU64> {
            Arc::clone(&self.written)
        }

        pub fn contents(&self) -> Arc<Mutex<Cursor<Vec<u8>>>> {
            Arc::clone(&self.inner)
        }
    }

    impl Write for FaultySink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if let Some(pause) = self.pause {
                std::thread::sleep(pause);
            }
            let so_far = self.written.load(Ordering::SeqCst);
            if so_far.saturating_add(buf.len() as u64) > self.fail_after {
                return Err(io::Error::new(self.kind, "injected write failure"));
            }
            let n = self
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .write(buf)?;
            self.written.fetch_add(n as u64, Ordering::SeqCst);
            Ok(n)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for FaultySink {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .seek(pos)
        }
    }

    impl Sink for FaultySink {
        fn sync(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
