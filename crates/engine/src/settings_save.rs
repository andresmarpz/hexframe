use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::settings::Settings;

use crate::api::{NoticeLevel, UiMsg};
use crate::control::Sink;

#[derive(Debug)]
pub(crate) struct SettingsSaver {
    tx: Option<Sender<Settings>>,
    thread: Option<JoinHandle<()>>,
}

impl SettingsSaver {
    pub(crate) fn spawn(path: Option<PathBuf>, sink: Sink) -> Self {
        let Some(path) = path else {
            return Self {
                tx: None,
                thread: None,
            };
        };
        let (tx, rx) = unbounded::<Settings>();
        let spawned = thread::Builder::new()
            .name("settings-save".into())
            .spawn(move || {
                for settings in rx {
                    if let Err(err) = settings.save(&path) {
                        tracing::warn!(path = %path.display(), %err, "cannot save settings");
                        sink(UiMsg::Notice {
                            level: NoticeLevel::Warn,
                            text: format!("Cannot save settings: {err}"),
                        });
                    }
                }
            });
        match spawned {
            Ok(thread) => Self {
                tx: Some(tx),
                thread: Some(thread),
            },
            Err(err) => {
                tracing::error!(%err, "cannot start settings save thread");
                Self {
                    tx: None,
                    thread: None,
                }
            }
        }
    }

    pub(crate) fn save(&self, settings: Settings) {
        if let Some(tx) = &self.tx
            && tx.send(settings).is_err()
        {
            tracing::warn!("settings saver is gone");
        }
    }
}

impl Drop for SettingsSaver {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}
