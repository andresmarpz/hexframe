use std::mem::size_of;
use std::time::{Duration, Instant};

use greyscreen_core::crop::{CropRect, ScreenRect, client_crop};
use greyscreen_core::media::CaptureTarget;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

const REFRESH_EVERY: Duration = Duration::from_millis(500);

#[derive(Debug)]
struct Cached {
    frame: (u32, u32),
    at: Instant,
    crop: CropRect,
}

#[derive(Debug)]
pub(crate) struct ClientArea {
    hwnd: Option<HWND>,
    cached: Option<Cached>,
}

impl ClientArea {
    pub(crate) fn for_target(target: &CaptureTarget) -> Self {
        let hwnd = match target {
            CaptureTarget::GameWindow { hwnd, .. } => Some(HWND(*hwnd as *mut core::ffi::c_void)),
            CaptureTarget::PrimaryMonitor => None,
        };
        Self { hwnd, cached: None }
    }

    pub(crate) fn crop(&mut self, frame: (u32, u32)) -> CropRect {
        let now = Instant::now();
        if let Some(cached) = &self.cached
            && cached.frame == frame
            && now.duration_since(cached.at) < REFRESH_EVERY
        {
            return cached.crop;
        }
        let crop = self.query(frame);
        let changed = self
            .cached
            .as_ref()
            .is_none_or(|cached| cached.crop != crop);
        if changed {
            tracing::info!(
                frame_width = frame.0,
                frame_height = frame.1,
                left = crop.left,
                top = crop.top,
                width = crop.width,
                height = crop.height,
                full = crop.is_full(frame),
                "capture crop"
            );
        }
        self.cached = Some(Cached {
            frame,
            at: now,
            crop,
        });
        crop
    }

    fn query(&self, frame: (u32, u32)) -> CropRect {
        let full = CropRect::full(frame);
        let Some(hwnd) = self.hwnd else {
            return full;
        };
        match read_metrics(hwnd) {
            Some(metrics) => client_crop(frame, metrics.bounds, metrics.origin, metrics.size),
            None => full,
        }
    }
}

struct Metrics {
    bounds: ScreenRect,
    origin: (i32, i32),
    size: (i32, i32),
}

fn read_metrics(hwnd: HWND) -> Option<Metrics> {
    let mut bounds = RECT::default();
    // SAFETY: `bounds` is a live RECT and the size passed is its exact size.
    let bounds_result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&raw mut bounds).cast(),
            size_of::<RECT>() as u32,
        )
    };
    if let Err(error) = bounds_result {
        tracing::debug!(%error, "window frame bounds unavailable");
        return None;
    }
    let mut client = RECT::default();
    // SAFETY: `client` is a live out value; the call fails cleanly for an invalid handle.
    if let Err(error) = unsafe { GetClientRect(hwnd, &mut client) } {
        tracing::debug!(%error, "window client rect unavailable");
        return None;
    }
    let mut origin = POINT { x: 0, y: 0 };
    // SAFETY: `origin` is a live POINT; the call reports failure for an invalid handle.
    if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
        tracing::debug!("window client origin unavailable");
        return None;
    }
    Some(Metrics {
        bounds: ScreenRect {
            left: bounds.left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        },
        origin: (origin.x, origin.y),
        size: (client.right - client.left, client.bottom - client.top),
    })
}
