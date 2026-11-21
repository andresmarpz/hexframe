use greyscreen_core::keybinds::{Action, KeyCombo};

#[cfg(windows)]
use crossbeam_channel::Sender;

#[cfg(windows)]
use crate::api::EngineCmd;
#[cfg(windows)]
use crate::control::Sink;

pub trait HotkeyBackend: Send {
    fn set(&self, bindings: &[(Action, KeyCombo)]);
}

#[cfg(windows)]
struct Windows {
    listener: greyscreen_capture::hotkey::Listener,
}

#[cfg(windows)]
impl HotkeyBackend for Windows {
    fn set(&self, bindings: &[(Action, KeyCombo)]) {
        self.listener.set(bindings);
    }
}

#[cfg(windows)]
pub(crate) fn platform(cmd: Sender<EngineCmd>, sink: Sink) -> Box<dyn HotkeyBackend> {
    use crate::api::UiMsg;
    use greyscreen_capture::hotkey::{Event, spawn};
    let listener = spawn(move |event| match event {
        Event::Pressed(action) => {
            if cmd.send(EngineCmd::GlobalAction(action)).is_err() {
                tracing::debug!("engine stopped before the hotkey was handled");
            }
        }
        Event::Applied(failed) => {
            let errors = failed
                .into_iter()
                .map(|failure| {
                    let why = if failure.in_use {
                        "is already used by another app"
                    } else {
                        "cannot be used"
                    };
                    tracing::warn!(action = failure.action.id(), combo = %failure.combo, in_use = failure.in_use, "cannot register a hotkey");
                    (
                        failure.action,
                        format!("{} {why}. Choose a different key.", failure.combo.display(false)),
                    )
                })
                .collect();
            sink(UiMsg::HotkeyStatus { errors });
        }
    });
    match listener {
        Ok(listener) => Box::new(Windows { listener }),
        Err(err) => {
            tracing::warn!(%err, "cannot start the hotkey thread");
            Box::new(Unavailable)
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn platform(
    _cmd: crossbeam_channel::Sender<crate::api::EngineCmd>,
    _sink: crate::control::Sink,
) -> Box<dyn HotkeyBackend> {
    Box::new(Unavailable)
}

struct Unavailable;

impl HotkeyBackend for Unavailable {
    fn set(&self, _bindings: &[(Action, KeyCombo)]) {}
}
