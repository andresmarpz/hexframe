mod fault;

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_mp4::fragmented::{FinishedInfo, FragmentedWriter};
use greyscreen_mp4::sink::Sink;

pub(crate) const PART_FILE: &str = "game.mp4.part";
pub(crate) const VIDEO_FILE: &str = "game.mp4";
const QUEUE: usize = 4096;
const RENAME_ATTEMPTS: u32 = 5;
const RENAME_PAUSE: Duration = Duration::from_millis(200);

pub(crate) type Report = Result<FinishedInfo, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sent {
    Queued,
    Full,
    Stopped,
}

#[derive(Debug)]
pub(crate) struct GameWriter {
    tx: Option<Sender<Arc<Packet>>>,
    thread: Option<JoinHandle<()>>,
}

impl GameWriter {
    pub(crate) fn spawn(
        folder: &Path,
        video: &VideoFormat,
        audio: Option<&AudioFormat>,
        on_failed: impl FnOnce(String) + Send + 'static,
        on_done: impl FnOnce(Report) + Send + 'static,
    ) -> Result<Self, String> {
        Self::spawn_with_queue(folder, video, audio, QUEUE, on_failed, on_done)
    }

    pub(crate) fn spawn_with_queue(
        folder: &Path,
        video: &VideoFormat,
        audio: Option<&AudioFormat>,
        queue: usize,
        on_failed: impl FnOnce(String) + Send + 'static,
        on_done: impl FnOnce(Report) + Send + 'static,
    ) -> Result<Self, String> {
        let part = folder.join(PART_FILE);
        match fault::limit() {
            Some(limit) => {
                let file = fault::LimitedFile::create(&part, limit).map_err(|e| e.to_string())?;
                let writer =
                    FragmentedWriter::with_sink(file, video, audio).map_err(|e| e.to_string())?;
                Self::spawn_writer(writer, folder, queue, on_failed, on_done)
            }
            None => {
                let writer =
                    FragmentedWriter::create(&part, video, audio).map_err(|e| e.to_string())?;
                Self::spawn_writer(writer, folder, queue, on_failed, on_done)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn spawn_on(
        sink: impl Sink + Send + 'static,
        folder: &Path,
        video: &VideoFormat,
        audio: Option<&AudioFormat>,
        queue: usize,
        on_failed: impl FnOnce(String) + Send + 'static,
        on_done: impl FnOnce(Report) + Send + 'static,
    ) -> Result<Self, String> {
        let writer = FragmentedWriter::with_sink(sink, video, audio).map_err(|e| e.to_string())?;
        Self::spawn_writer(writer, folder, queue, on_failed, on_done)
    }

    fn spawn_writer(
        writer: FragmentedWriter<impl Sink + Send + 'static>,
        folder: &Path,
        queue: usize,
        on_failed: impl FnOnce(String) + Send + 'static,
        on_done: impl FnOnce(Report) + Send + 'static,
    ) -> Result<Self, String> {
        let part = folder.join(PART_FILE);
        let target = folder.join(VIDEO_FILE);
        let (tx, rx) = bounded(queue);
        let thread = thread::Builder::new()
            .name("game-writer".into())
            .spawn(move || {
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    drain(writer, &rx, &part, &target, on_failed)
                }));
                let report = outcome.unwrap_or_else(|_| {
                    tracing::error!("game writer thread panicked");
                    Err("game writer thread panicked".to_owned())
                });
                on_done(report);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            tx: Some(tx),
            thread: Some(thread),
        })
    }

    pub(crate) fn send(&mut self, packet: Arc<Packet>) -> Sent {
        let Some(tx) = &self.tx else {
            return Sent::Stopped;
        };
        match tx.try_send(packet) {
            Ok(()) => Sent::Queued,
            Err(TrySendError::Full(_)) => {
                tracing::error!("game writer queue is full, stopping the full game recording");
                self.tx.take();
                Sent::Full
            }
            Err(TrySendError::Disconnected(_)) => {
                tracing::warn!("game writer is gone, dropping packet");
                self.tx.take();
                Sent::Stopped
            }
        }
    }

    pub(crate) fn queued(&self) -> u32 {
        self.tx
            .as_ref()
            .map_or(0, |tx| u32::try_from(tx.len()).unwrap_or(u32::MAX))
    }

    pub(crate) fn close(&mut self) {
        self.tx.take();
    }
}

impl Drop for GameWriter {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

fn drain(
    mut writer: FragmentedWriter<impl Sink>,
    rx: &Receiver<Arc<Packet>>,
    part: &Path,
    target: &Path,
    on_failed: impl FnOnce(String),
) -> Report {
    let mut on_failed = Some(on_failed);
    let mut failure: Option<String> = None;
    let mut seen_key = false;
    for packet in rx {
        if failure.is_some() {
            continue;
        }
        if packet.track == Track::Video {
            seen_key |= packet.key;
            if !seen_key {
                continue;
            }
        }
        if let Err(err) = writer.push(&packet) {
            tracing::error!(%err, "game writer push failed");
            let text = err.to_string();
            if let Some(notify) = on_failed.take() {
                notify(text.clone());
            }
            failure = Some(text);
        }
    }
    let Some(first) = failure else {
        let info = writer.finish().map_err(|e| e.to_string())?;
        rename_with_retry(part, target)?;
        return Ok(info);
    };
    match writer.finish() {
        Ok(info) => rename_with_retry(part, target).map(|()| info),
        Err(err) => {
            tracing::warn!(%err, "cannot finish the game video after a write failure");
            Err(first)
        }
    }
}

fn rename_with_retry(part: &Path, target: &Path) -> Result<(), String> {
    let mut last = String::new();
    for attempt in 1..=RENAME_ATTEMPTS {
        match fs::rename(part, target) {
            Ok(()) => return Ok(()),
            Err(err) => {
                tracing::warn!(attempt, %err, "cannot rename the finished game video");
                last = err.to_string();
            }
        }
        if attempt < RENAME_ATTEMPTS {
            thread::sleep(RENAME_PAUSE);
        }
    }
    Err(last)
}

pub use fault::set_write_limit;

#[cfg(test)]
mod tests;
