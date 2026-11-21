use crossbeam_channel::Sender;
use greyscreen_core::settings::Hotkey;

use crate::api::EngineCmd;
use crate::control::Sink;

#[cfg(windows)]
pub(crate) struct Hotkeys {
    listener: Option<greyscreen_capture::hotkey::Listener>,
}

#[cfg(windows)]
impl Hotkeys {
    pub(crate) fn start(cmd: Sender<EngineCmd>, sink: Sink) -> Self {
        use crate::api::UiMsg;
        use greyscreen_capture::hotkey::{Event, spawn};
        let listener = spawn(move |event| match event {
            Event::Pressed => {
                if cmd.send(EngineCmd::HotkeyPressed).is_err() {
                    tracing::debug!("engine stopped before the hotkey was handled");
                }
            }
            Event::Registered(hotkey) => {
                tracing::info!(hotkey = ?hotkey.map(|h| h.to_string()), "save clip hotkey registered");
                sink(UiMsg::HotkeyStatus { error: None });
            }
            Event::Failed { hotkey, in_use } => {
                tracing::warn!(%hotkey, in_use, "cannot register the save clip hotkey");
                let why = if in_use {
                    "is already used by another app"
                } else {
                    "cannot be used"
                };
                sink(UiMsg::HotkeyStatus {
                    error: Some(format!("{hotkey} {why}. Choose a different key.")),
                });
            }
        });
        match listener {
            Ok(listener) => Self {
                listener: Some(listener),
            },
            Err(err) => {
                tracing::warn!(%err, "cannot start the hotkey thread");
                Self { listener: None }
            }
        }
    }

    pub(crate) fn set(&self, hotkey: Option<&Hotkey>) {
        if let Some(listener) = &self.listener {
            listener.set(hotkey);
        }
    }
}

#[cfg(not(windows))]
pub(crate) struct Hotkeys;

#[cfg(not(windows))]
impl Hotkeys {
    pub(crate) fn start(_cmd: Sender<EngineCmd>, _sink: Sink) -> Self {
        Hotkeys
    }

    pub(crate) fn set(&self, _hotkey: Option<&Hotkey>) {}
}
