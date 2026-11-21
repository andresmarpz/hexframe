use rune::{AppHandle, Tray, TrayEvent, TrayIcon, TrayItem, TraySpec};

use crate::ui::Ui;

const ICON_SIZE: u32 = 32;
const OPEN_ID: &str = "open";
const QUIT_ID: &str = "quit";

pub(crate) fn start(handle: AppHandle<Ui>) -> Option<Tray> {
    let spec = TraySpec::new("dev.greyscreen.app", "Greyscreen", icon())
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

fn icon() -> TrayIcon {
    let size = ICON_SIZE as f32;
    let radius = size * 0.22;
    let mut pixels = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let inside = inside_rounded_square(x as f32 + 0.5, y as f32 + 0.5, size, radius);
            let alpha = if inside { 255 } else { 0 };
            pixels.extend_from_slice(&[124, 77, 255, alpha]);
        }
    }
    TrayIcon::rgba(ICON_SIZE, ICON_SIZE, pixels)
}

fn inside_rounded_square(x: f32, y: f32, size: f32, radius: f32) -> bool {
    let dx = (x - size / 2.0).abs() - (size / 2.0 - radius);
    let dy = (y - size / 2.0).abs() - (size / 2.0 - radius);
    dx.max(0.0).hypot(dy.max(0.0)) <= radius
}
