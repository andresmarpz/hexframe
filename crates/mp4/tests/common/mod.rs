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

#[allow(dead_code)]
pub mod tables {
    use greyscreen_mp4::reader::{BoxRef, find_box};

    pub struct TrackView {
        pub handler: String,
        pub trak_children: Vec<String>,
        pub stbl_children: Vec<String>,
        pub sample_count: u32,
        pub durations: Vec<u32>,
        pub chunk_counts: Vec<u32>,
    }

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    fn names(data: &[u8], parent: &BoxRef) -> Vec<String> {
        parent
            .children(data)
            .map(|b| String::from_utf8_lossy(&b.fourcc).into_owned())
            .collect()
    }

    fn payload<'a>(data: &'a [u8], parent: &BoxRef, fourcc: &[u8; 4]) -> Option<&'a [u8]> {
        parent
            .find(data, fourcc)
            .map(|b| &b.payload_bytes(data)[4..])
    }

    pub fn tracks(file: &[u8]) -> Vec<TrackView> {
        let moov = find_box(file, b"moov").unwrap_or_else(|| panic!("no moov"));
        moov.children(file)
            .filter(|b| &b.fourcc == b"trak")
            .map(|trak| view(file, &trak))
            .collect()
    }

    fn view(file: &[u8], trak: &BoxRef) -> TrackView {
        let mdia = trak
            .find(file, b"mdia")
            .unwrap_or_else(|| panic!("no mdia"));
        let minf = mdia
            .find(file, b"minf")
            .unwrap_or_else(|| panic!("no minf"));
        let stbl = minf
            .find(file, b"stbl")
            .unwrap_or_else(|| panic!("no stbl"));
        let hdlr = payload(file, &mdia, b"hdlr").unwrap_or_default();
        let stsz = payload(file, &stbl, b"stsz").unwrap_or_default();
        let stts = payload(file, &stbl, b"stts").unwrap_or_default();
        let stsc = payload(file, &stbl, b"stsc").unwrap_or_default();
        let co64 = payload(file, &stbl, b"co64").unwrap_or_default();
        let mut durations = Vec::new();
        for entry in 0..u32_at(stts, 0) as usize {
            let (count, delta) = (u32_at(stts, 4 + entry * 8), u32_at(stts, 8 + entry * 8));
            durations.extend(std::iter::repeat_n(delta, count as usize));
        }
        let chunks = u32_at(co64, 0) as usize;
        let runs: Vec<(u32, u32)> = (0..u32_at(stsc, 0) as usize)
            .map(|e| (u32_at(stsc, 4 + e * 12), u32_at(stsc, 8 + e * 12)))
            .collect();
        let chunk_counts = (1..=chunks as u32)
            .map(|chunk| runs.iter().rev().find(|r| r.0 <= chunk).map_or(0, |r| r.1))
            .collect();
        TrackView {
            handler: String::from_utf8_lossy(&hdlr[4..8]).into_owned(),
            trak_children: names(file, trak),
            stbl_children: names(file, &stbl),
            sample_count: u32_at(stsz, 4),
            durations,
            chunk_counts,
        }
    }
}
