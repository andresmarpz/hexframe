mod view;

use std::io::Cursor;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use greyscreen_core::time::mono_now;
use greyscreen_engine::EngineCmd;
use rune::{AppHandle, Tray, TrayEvent, TrayIcon, TraySpec};

use crate::ui::Ui;
pub(crate) use view::TrayState;
use view::{OPEN_ID, QUIT_ID, SAVE_ID, TOGGLE_ID, View};

const TRAY_PNG: &[u8] = include_bytes!("../assets/tray.png");
const TRAY_RECORDING_PNG: &[u8] = include_bytes!("../assets/tray-recording.png");
const TOOLTIP_EVERY: Duration = Duration::from_secs(1);

type Events = Arc<dyn Fn(TrayEvent) + Send + Sync>;

#[derive(Debug, Clone)]
pub(crate) struct Link {
    tx: Sender<TrayState>,
}

impl Link {
    pub(crate) fn send(&self, state: TrayState) {
        if self.tx.send(state).is_err() {
            tracing::debug!("tray thread ended");
        }
    }

    #[cfg(test)]
    pub(crate) fn detached() -> (Link, Receiver<TrayState>) {
        let (tx, rx) = unbounded();
        (Link { tx }, rx)
    }
}

pub(crate) fn start(handle: AppHandle<Ui>, initial: TrayState) -> Option<Link> {
    let events = events(handle);
    let shown = View::of(&initial, mono_now());
    let tray = build(&shown, &events)?;
    let (tx, rx) = unbounded();
    let spawned = thread::Builder::new()
        .name("tray".into())
        .spawn(move || run(&rx, tray, shown, initial, &events));
    match spawned {
        Ok(_) => Some(Link { tx }),
        Err(err) => {
            tracing::warn!(%err, "cannot start the tray thread");
            None
        }
    }
}

fn events(handle: AppHandle<Ui>) -> Events {
    Arc::new(move |event| match event {
        TrayEvent::Menu(id) => match id.as_str() {
            OPEN_ID => handle.open(),
            TOGGLE_ID => {
                handle.update(|ui: &mut Ui, _| ui.change(|s| s.auto_record = !s.auto_record))
            }
            SAVE_ID => handle.update(|ui: &mut Ui, _| ui.send(EngineCmd::SaveClipNow)),
            QUIT_ID => handle.quit(),
            _ => {}
        },
        TrayEvent::Activate { .. } => handle.open(),
        _ => {}
    })
}

fn run(
    rx: &Receiver<TrayState>,
    first: Tray,
    mut shown: View,
    mut state: TrayState,
    events: &Events,
) {
    let mut tray = Some(first);
    loop {
        let next = if state.is_recording() {
            rx.recv_timeout(TOOLTIP_EVERY)
        } else {
            rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
        };
        match next {
            Ok(next) => state = rx.try_iter().last().unwrap_or(next),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let view = View::of(&state, mono_now());
        if view.recording != shown.recording || tray.is_none() {
            // The old icon must leave before the new one appears. Tray has a Drop on Windows only.
            #[allow(clippy::drop_non_drop)]
            drop(tray.take());
            tray = build(&view, events);
        } else if let Some(tray) = &tray {
            if view.tooltip != shown.tooltip {
                tray.set_tooltip(view.tooltip.clone());
            }
            if view.items != shown.items {
                tray.set_menu(view.items.clone());
            }
        }
        shown = view;
    }
}

fn build(view: &View, events: &Events) -> Option<Tray> {
    let png = if view.recording {
        TRAY_RECORDING_PNG
    } else {
        TRAY_PNG
    };
    let Some(icon) = icon(png) else {
        tracing::warn!("tray icon cannot be decoded");
        return None;
    };
    let spec = TraySpec::new("dev.greyscreen.app", "Greyscreen", icon)
        .tooltip(view.tooltip.clone())
        .menu(view.items.clone());
    let events = Arc::clone(events);
    match Tray::new(spec, move |event| events(event)) {
        Ok(tray) => Some(tray),
        Err(err) => {
            tracing::warn!(%err, "tray unavailable");
            None
        }
    }
}

fn icon(png: &[u8]) -> Option<TrayIcon> {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(TrayIcon::rgba(info.width, info.height, buf))
}

#[cfg(test)]
mod tests;
