use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use greyscreen_mp4::sink::Sink;

const OFF: u64 = u64::MAX;

static LIMIT: AtomicU64 = AtomicU64::new(OFF);

pub fn set_write_limit(bytes: Option<u64>) {
    LIMIT.store(bytes.unwrap_or(OFF), Ordering::SeqCst);
}

static DELAY_MS: AtomicU64 = AtomicU64::new(0);

const WRITER_THREAD: &str = "game-writer";
const DELAY_SLICE: Duration = Duration::from_millis(20);

pub fn set_write_delay(delay: Option<Duration>) {
    let millis = delay.map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    DELAY_MS.store(millis, Ordering::SeqCst);
}

pub(super) fn active() -> bool {
    limit().is_some() || DELAY_MS.load(Ordering::SeqCst) > 0
}

fn stall_writer_thread() {
    if thread::current().name() != Some(WRITER_THREAD) {
        return;
    }
    let delay = Duration::from_millis(DELAY_MS.load(Ordering::SeqCst));
    let until = Instant::now() + delay;
    while DELAY_MS.load(Ordering::SeqCst) > 0 && Instant::now() < until {
        thread::sleep(DELAY_SLICE);
    }
}

pub(super) fn limit() -> Option<u64> {
    match LIMIT.load(Ordering::SeqCst) {
        OFF => None,
        bytes => Some(bytes),
    }
}

pub(super) struct LimitedFile {
    file: File,
    left: u64,
}

impl LimitedFile {
    pub(super) fn create(path: &Path, limit: u64) -> io::Result<Self> {
        Ok(Self {
            file: File::create(path)?,
            left: limit,
        })
    }
}

impl Write for LimitedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        stall_writer_thread();
        let len = buf.len() as u64;
        if len > self.left {
            return Err(io::Error::from(io::ErrorKind::StorageFull));
        }
        self.left -= len;
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for LimitedFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Sink for LimitedFile {
    fn sync(&mut self) -> io::Result<()> {
        self.file.sync_data()
    }
}
