use std::io::Cursor;

use rune::{AppHandle, Tray, TrayEvent, TrayIcon, TrayItem, TraySpec};

use crate::ui::Ui;

const TRAY_PNG: &[u8] = include_bytes!("../assets/tray.png");
const OPEN_ID: &str = "open";
const QUIT_ID: &str = "quit";

pub(crate) fn start(handle: AppHandle<Ui>) -> Option<Tray> {
    let Some(icon) = icon() else {
        tracing::warn!("tray icon cannot be decoded");
        return None;
    };
    let spec = TraySpec::new("dev.greyscreen.app", "Greyscreen", icon)
        .tooltip("Greyscreen")
        .menu(vec![
            TrayItem::action(OPEN_ID, "Open"),
            TrayItem::action(QUIT_ID, "Quit"),
        ]);
    let on_event = move |event: TrayEvent| match event {
        TrayEvent::Menu(id) if id == QUIT_ID => handle.quit(),
        TrayEvent::Menu(id) if id == OPEN_ID => handle.open(),
        TrayEvent::Activate { .. } => handle.open(),
        _ => {}
    };
    match Tray::new(spec, on_event) {
        Ok(tray) => Some(tray),
        Err(err) => {
            tracing::warn!(%err, "tray unavailable");
            None
        }
    }
}

fn icon() -> Option<TrayIcon> {
    let mut reader = png::Decoder::new(Cursor::new(TRAY_PNG)).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(TrayIcon::rgba(info.width, info.height, buf))
}
