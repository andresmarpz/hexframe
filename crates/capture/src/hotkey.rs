use std::io;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::keybinds::{Action, KeyCombo};
use thiserror::Error;
use windows::Win32::Foundation::{ERROR_HOTKEY_ALREADY_REGISTERED, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
    UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WM_APP, WM_HOTKEY, WM_QUIT,
    WM_USER,
};

const WM_APPLY: u32 = WM_APP + 1;

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("cannot start the hotkey thread: {0}")]
    Thread(#[from] io::Error),
    #[error("the hotkey thread stopped before it was ready")]
    NotReady,
}

pub type Bindings = Vec<(Action, KeyCombo)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub action: Action,
    pub combo: KeyCombo,
    pub in_use: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Pressed(Action),
    Applied(Vec<Failure>),
}

#[derive(Debug)]
pub struct Listener {
    thread_id: u32,
    wanted: Sender<Bindings>,
    thread: Option<JoinHandle<()>>,
}

pub fn spawn(on_event: impl Fn(Event) + Send + 'static) -> Result<Listener, HotkeyError> {
    let (wanted, wanted_rx) = unbounded();
    let (ready_tx, ready_rx) = mpsc::channel();
    let thread = thread::Builder::new()
        .name("hotkey".into())
        .spawn(move || run(&wanted_rx, &on_event, &ready_tx))?;
    let thread_id = ready_rx.recv().map_err(|_| HotkeyError::NotReady)?;
    Ok(Listener {
        thread_id,
        wanted,
        thread: Some(thread),
    })
}

impl Listener {
    pub fn set(&self, bindings: &[(Action, KeyCombo)]) {
        if self.wanted.send(bindings.to_vec()).is_err() {
            tracing::warn!("the hotkey thread is gone");
            return;
        }
        self.post(WM_APPLY);
    }

    fn post(&self, message: u32) {
        // SAFETY: the thread id came from the hotkey thread, which created its queue first.
        let posted = unsafe { PostThreadMessageW(self.thread_id, message, WPARAM(0), LPARAM(0)) };
        if let Err(error) = posted {
            tracing::warn!(%error, "cannot reach the hotkey thread");
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.post(WM_QUIT);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("hotkey thread panicked");
        }
    }
}

fn run(wanted: &Receiver<Bindings>, on_event: &dyn Fn(Event), ready: &mpsc::Sender<u32>) {
    let mut msg = MSG::default();
    // SAFETY: msg is a valid out pointer. Peeking makes Windows create this thread's queue.
    unsafe { PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE) }.as_bool();
    // SAFETY: reads the id of the calling thread.
    let thread_id = unsafe { GetCurrentThreadId() };
    if ready.send(thread_id).is_err() {
        return;
    }
    let mut registered: Vec<Action> = Vec::new();
    loop {
        // SAFETY: msg is a valid out pointer and the filter takes every message.
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if got.0 <= 0 {
            break;
        }
        match msg.message {
            WM_HOTKEY => {
                if let Some(action) = hotkey_action(&registered, msg.wParam.0) {
                    on_event(Event::Pressed(action));
                }
            }
            WM_APPLY => {
                while let Ok(next) = wanted.try_recv() {
                    unregister_all(registered.len());
                    let (now, failed) = register_all(&next);
                    registered = now;
                    on_event(Event::Applied(failed));
                }
            }
            _ => {}
        }
    }
    unregister_all(registered.len());
}

fn hotkey_action(registered: &[Action], id: usize) -> Option<Action> {
    registered.get(id.checked_sub(1)?).copied()
}

fn register_all(bindings: &[(Action, KeyCombo)]) -> (Vec<Action>, Vec<Failure>) {
    let mut registered = Vec::new();
    let mut failed = Vec::new();
    for (action, combo) in bindings {
        let id = i32::try_from(registered.len() + 1).unwrap_or(i32::MAX);
        match register(id, combo) {
            Ok(()) => registered.push(*action),
            Err(in_use) => failed.push(Failure {
                action: *action,
                combo: combo.clone(),
                in_use,
            }),
        }
    }
    (registered, failed)
}

fn modifiers(combo: &KeyCombo) -> HOT_KEY_MODIFIERS {
    let mut mods = MOD_NOREPEAT;
    for (held, flag) in [
        (combo.ctrl, MOD_CONTROL),
        (combo.alt, MOD_ALT),
        (combo.shift, MOD_SHIFT),
        (combo.meta, MOD_WIN),
    ] {
        if held {
            mods |= flag;
        }
    }
    mods
}

fn register(id: i32, combo: &KeyCombo) -> Result<(), bool> {
    // SAFETY: registers on this thread's queue with no window. The id is unique among live ones.
    unsafe { RegisterHotKey(None, id, modifiers(combo), combo.virtual_key()) }
        .map_err(|error| error.code() == ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult())
}

fn unregister_all(count: usize) {
    for id in 1..=count {
        let id = i32::try_from(id).unwrap_or(i32::MAX);
        // SAFETY: only called on the thread that registered the ids.
        if let Err(error) = unsafe { UnregisterHotKey(None, id) } {
            tracing::warn!(%error, "cannot unregister a hotkey");
        }
    }
}
