//! A video's sound, read ahead of the clock on a source reader and a thread of its own. The
//! decoder's reader can't do it: it decodes only a few pictures ahead, and on the layer it waits
//! for each picture's time, while the sound output needs more than 100 ms queued at every write.

use super::audio::{Queue, push_sample};
use super::mf::{self, Decode, Sound};
use super::super::{Kind, MediaSource};
use std::sync::Arc;
use std::time::Duration;
use windows::Win32::Media::MediaFoundation::{MF_SOURCE_READERF_ENDOFSTREAM, MF_SOURCE_READERF_ERROR, IMFSourceReader};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// How long a reader with a full queue sleeps when nothing wakes it.
const PATIENCE: Duration = Duration::from_millis(250);

/// Where a video's sound comes from.
pub(super) struct Read {
    pub source: MediaSource,
    /// Unwrapped seconds to start at.
    pub at: f64,
    pub duration: f64,
}

/// Starts the reader thread; it runs until the output stops.
pub(super) fn start(read: Read, queue: Arc<Queue>, sound: Sound) -> Result<(), String> {
    std::thread::Builder::new()
        .name("rune-video-sound-read".into())
        .spawn(move || {
            // SAFETY: COM for this thread, matched below.
            let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
            if let Err(e) = run(&read, &queue, sound) {
                eprintln!("rune: the clip's sound stopped: {e}");
            }
            if com {
                // SAFETY: matches the initialisation above; the reader is gone.
                unsafe { CoUninitialize() };
            }
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// A seek's place on an unwrapped time: the loops before it, and the clip's own seconds.
fn place(at: f64, duration: f64) -> (f64, f64) {
    if duration > 0.0 {
        let epoch = (at / duration).floor() * duration;
        (epoch, at - epoch)
    } else {
        (0.0, at)
    }
}

struct Reader {
    reader: IMFSourceReader,
    index: u32,
    /// Added to stream times: the loops so far times the duration.
    epoch: f64,
    /// Sound before this unwrapped time (a seek's pre-roll) is dropped.
    skip_before: f64,
    /// Not at the stream's end.
    open: bool,
    /// A sample came since the last seek: a loop has sound to go back to.
    got: bool,
}

impl Reader {
    fn seek(&mut self, at: f64, duration: f64) -> Result<(), String> {
        let (epoch, to) = place(at, duration);
        mf::seek(&self.reader, to)?;
        (self.epoch, self.skip_before, self.open, self.got) = (epoch, at, true, false);
        Ok(())
    }
}

fn run(read: &Read, queue: &Queue, sound: Sound) -> Result<(), String> {
    let lib = mf::library()?;
    let source = lib.resolve(&read.source)?;
    let opened = mf::open(lib, &source, Kind::Audio, Decode::Cpu, true)?;
    let index = opened.audio.ok_or("no audio stream")?;
    if opened.sound != Some(sound) {
        return Err("the sound's format differs from the clip's".into());
    }
    let mut r = Reader { reader: opened.reader, index, epoch: 0.0, skip_before: 0.0, open: true, got: false };
    r.seek(read.at, read.duration)?;
    loop {
        let mut feed = queue.lock();
        loop {
            if feed.stop {
                return Ok(());
            }
            if let Some(to) = feed.seek.take() {
                feed.clear();
                drop(feed);
                r.seek(to, read.duration)?;
                feed = queue.lock();
                continue;
            }
            // Looping turned on after the sound's end: on into the next loop.
            if !r.open && r.got && feed.looping && read.duration > 0.0 {
                let next = r.epoch + read.duration;
                drop(feed);
                r.seek(next, read.duration)?;
                feed = queue.lock();
                continue;
            }
            if r.open && !queue.full(&feed, sound) {
                break;
            }
            feed = queue.wait(feed, PATIENCE);
        }
        let looping = feed.looping;
        drop(feed);
        let mut got = mf::read(&r.reader, r.index)?;
        if got.flags & MF_SOURCE_READERF_ERROR.0 != 0 {
            return Err("the decoder stopped with an error".into());
        }
        if let Some(sample) = got.sample.take() {
            r.got = true;
            push_sample(queue, sound, &sample, r.epoch + got.time, r.skip_before);
        }
        if got.flags & MF_SOURCE_READERF_ENDOFSTREAM.0 != 0 {
            if looping && r.got && read.duration > 0.0 {
                let next = r.epoch + read.duration;
                r.seek(next, read.duration)?;
            } else {
                r.open = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::place;

    #[test]
    fn a_seek_on_an_unwrapped_time_lands_in_its_loop() {
        assert_eq!(place(25.0, 10.0), (20.0, 5.0));
        assert_eq!(place(7.5, 10.0), (0.0, 7.5));
        assert_eq!(place(7.5, 0.0), (0.0, 7.5));
    }
}
