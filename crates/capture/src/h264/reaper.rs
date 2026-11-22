use std::sync::mpsc::{SendError, Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::Win32::Media::MediaFoundation::{
    ICodecAPI, IMFActivate, IMFMediaEventGenerator, IMFTransform,
};

use crate::com::ComGuard;

const GRACE: Duration = Duration::from_secs(2);

pub(super) struct Parked {
    pub(super) _transform: IMFTransform,
    pub(super) _codec: ICodecAPI,
    pub(super) _events: Option<IMFMediaEventGenerator>,
    pub(super) _activate: Option<IMFActivate>,
}

struct Entry {
    release_at: Instant,
    _parked: Parked,
}

// SAFETY: the transform is free-threaded and the thread that releases it joins the MTA first.
unsafe impl Send for Entry {}

pub(super) fn park(parked: Parked) {
    static QUEUE: OnceLock<Option<Mutex<Sender<Entry>>>> = OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (sender, receiver) = channel::<Entry>();
        let spawned = std::thread::Builder::new()
            .name("mft-reaper".into())
            .spawn(move || {
                let _com = ComGuard::mta();
                while let Ok(entry) = receiver.recv() {
                    std::thread::sleep(entry.release_at.saturating_duration_since(Instant::now()));
                    drop(entry);
                }
            });
        spawned.ok().map(|_| Mutex::new(sender))
    });
    let entry = Entry {
        release_at: Instant::now() + GRACE,
        _parked: parked,
    };
    let sent = match queue {
        Some(sender) => sender
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .send(entry),
        None => Err(SendError(entry)),
    };
    if let Err(SendError(entry)) = sent {
        std::mem::forget(entry);
    }
}
