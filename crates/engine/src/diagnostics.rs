use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Sender, TrySendError, bounded};
use serde::Serialize;

pub(crate) const DIAG_FILE: &str = "diagnostics.jsonl";
const QUEUE: usize = 256;

#[derive(Debug)]
pub(crate) struct DiagWriter {
    tx: Option<Sender<String>>,
    thread: Option<JoinHandle<()>>,
}

impl DiagWriter {
    pub(crate) fn start(folder: &Path) -> Result<Self, String> {
        let file = File::create(folder.join(DIAG_FILE)).map_err(|e| e.to_string())?;
        let (tx, rx) = bounded::<String>(QUEUE);
        let thread = thread::Builder::new()
            .name("diag-writer".into())
            .spawn(move || {
                let mut out = BufWriter::new(file);
                for line in rx {
                    if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
                        tracing::warn!("diagnostics file write failed, stopping diagnostics");
                        return;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            tx: Some(tx),
            thread: Some(thread),
        })
    }

    pub(crate) fn line(&self, kind: &str, value: &impl Serialize) {
        let Some(tx) = &self.tx else {
            return;
        };
        let Ok(mut json) = serde_json::to_value(value) else {
            return;
        };
        if let Some(map) = json.as_object_mut() {
            map.insert("type".to_owned(), kind.into());
        }
        match tx.try_send(json.to_string()) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => tracing::debug!("diagnostics queue full, line dropped"),
        }
    }
}

impl Drop for DiagWriter {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

#[cfg(test)]
mod tests;
