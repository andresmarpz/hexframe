use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, bounded};
use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};
use greyscreen_mp4::fragmented::{FinishedInfo, FragmentedWriter};

pub(crate) const PART_FILE: &str = "game.mp4.part";
pub(crate) const VIDEO_FILE: &str = "game.mp4";
const QUEUE: usize = 4096;

pub(crate) type Report = Result<FinishedInfo, String>;

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
        on_done: impl FnOnce(Report) + Send + 'static,
    ) -> Result<Self, String> {
        let part = folder.join(PART_FILE);
        let writer = FragmentedWriter::create(&part, video, audio).map_err(|e| e.to_string())?;
        let target = folder.join(VIDEO_FILE);
        let (tx, rx) = bounded(QUEUE);
        let thread = thread::Builder::new()
            .name("game-writer".into())
            .spawn(move || {
                let outcome = catch_unwind(AssertUnwindSafe(|| drain(writer, &rx, &part, &target)));
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

    pub(crate) fn send(&self, packet: Arc<Packet>) {
        if let Some(tx) = &self.tx
            && tx.send(packet).is_err()
        {
            tracing::warn!("game writer is gone, dropping packet");
        }
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
    mut writer: FragmentedWriter,
    rx: &Receiver<Arc<Packet>>,
    part: &Path,
    target: &Path,
) -> Report {
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
            failure = Some(err.to_string());
        }
    }
    if let Some(err) = failure {
        return Err(err);
    }
    let info = writer.finish().map_err(|e| e.to_string())?;
    fs::rename(part, target).map_err(|e| e.to_string())?;
    Ok(info)
}
