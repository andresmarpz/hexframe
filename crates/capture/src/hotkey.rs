use std::io;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::settings::Hotkey;
use thiserror::Error;
use windows::Win32::Foundation::{ERROR_HOTKEY_ALREADY_REGISTERED, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey,
    UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WM_APP, WM_HOTKEY, WM_QUIT,
    WM_USER,
};

const HOTKEY_ID: i32 = 1;
const WM_APPLY: u32 = WM_APP + 1;

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("cannot start the hotkey thread: {0}")]
    Thread(#[from] io::Error),
    #[error("the hotkey thread stopped before it was ready")]
    NotReady,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Pressed,
    Registered(Option<Hotkey>),
    Failed { hotkey: Hotkey, in_use: bool },
}

#[derive(Debug)]
pub struct Listener {
    thread_id: u32,
    wanted: Sender<Option<Hotkey>>,
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
    pub fn set(&self, hotkey: Option<&Hotkey>) {
        if self.wanted.send(hotkey.cloned()).is_err() {
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

fn run(wanted: &Receiver<Option<Hotkey>>, on_event: &dyn Fn(Event), ready: &mpsc::Sender<u32>) {
    let mut msg = MSG::default();
    // SAFETY: msg is a valid out pointer. Peeking makes Windows create this thread's queue.
    unsafe { PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE) }.as_bool();
    // SAFETY: reads the id of the calling thread.
    let thread_id = unsafe { GetCurrentThreadId() };
    if ready.send(thread_id).is_err() {
        return;
    }
    let mut current: Option<Hotkey> = None;
    loop {
        // SAFETY: msg is a valid out pointer and the filter takes every message.
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if got.0 <= 0 {
            break;
        }
        match msg.message {
            WM_HOTKEY if msg.wParam.0 == HOTKEY_ID as usize => on_event(Event::Pressed),
            WM_APPLY => {
                while let Ok(next) = wanted.try_recv() {
                    current = apply(current.take(), next, on_event);
                }
            }
            _ => {}
        }
    }
    if current.is_some() {
        unregister();
    }
}

fn apply(
    registered: Option<Hotkey>,
    next: Option<Hotkey>,
    on_event: &dyn Fn(Event),
) -> Option<Hotkey> {
    if registered.is_some() {
        unregister();
    }
    let Some(hotkey) = next else {
        on_event(Event::Registered(None));
        return None;
    };
    match register(&hotkey) {
        Ok(()) => {
            on_event(Event::Registered(Some(hotkey.clone())));
            Some(hotkey)
        }
        Err(in_use) => {
            on_event(Event::Failed { hotkey, in_use });
            None
        }
    }
}

fn modifiers(hotkey: &Hotkey) -> HOT_KEY_MODIFIERS {
    let mut mods = MOD_NOREPEAT;
    for (held, flag) in [
        (hotkey.ctrl, MOD_CONTROL),
        (hotkey.alt, MOD_ALT),
        (hotkey.shift, MOD_SHIFT),
    ] {
        if held {
            mods |= flag;
        }
    }
    mods
}

fn register(hotkey: &Hotkey) -> Result<(), bool> {
    // SAFETY: registers on this thread's queue with no window. The id is unique here.
    unsafe { RegisterHotKey(None, HOTKEY_ID, modifiers(hotkey), hotkey.virtual_key()) }
        .map_err(|error| error.code() == ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult())
}

fn unregister() {
    // SAFETY: only called on the thread that registered the id.
    if let Err(error) = unsafe { UnregisterHotKey(None, HOTKEY_ID) } {
        tracing::warn!(%error, "cannot unregister the hotkey");
    }
}
