use windows::Foundation::TimeSpan;
use windows::Graphics::Capture::{
    GraphicsCaptureAccess, GraphicsCaptureAccessKind, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Security::Authorization::AppCapabilityAccess::AppCapabilityAccessStatus;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{HMONITOR, MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::factory;

use greyscreen_core::media::{CaptureTarget, capture_min_update_interval_hns};

use super::WgcOptions;

pub(super) fn create_item(target: &CaptureTarget) -> windows::core::Result<GraphicsCaptureItem> {
    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    match target {
        CaptureTarget::GameWindow { hwnd, .. } => {
            let hwnd = HWND(*hwnd as *mut core::ffi::c_void);
            // SAFETY: CreateForWindow validates the handle and returns an error for an invalid one.
            unsafe { interop.CreateForWindow(hwnd) }
        }
        CaptureTarget::PrimaryMonitor => {
            // SAFETY: MonitorFromPoint takes a value point and always returns a monitor with DEFAULTTOPRIMARY.
            let monitor: HMONITOR =
                unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
            // SAFETY: CreateForMonitor validates the handle and returns an error for an invalid one.
            unsafe { interop.CreateForMonitor(monitor) }
        }
    }
}

pub(super) fn configure_session(session: &GraphicsCaptureSession, options: WgcOptions) -> bool {
    let WgcOptions {
        fps,
        cursor,
        display_hz,
    } = options;
    tracing::info!(cursor, "graphics capture cursor");
    if let Err(error) = session.SetIsCursorCaptureEnabled(cursor) {
        tracing::warn!(%error, cursor, "set cursor capture failed");
    }
    tracing::info!(step = "borderless access", "graphics capture");
    request_borderless();
    let border_hidden =
        match fault!(CaptureBorder).and_then(|()| session.SetIsBorderRequired(false)) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(%error, "disable capture border failed");
                false
            }
        };
    let interval = TimeSpan {
        Duration: capture_min_update_interval_hns(fps, display_hz),
    };
    tracing::info!(
        interval_ms = interval.Duration as f64 / 10_000.0,
        display_hz,
        "graphics capture update interval"
    );
    if let Err(error) =
        fault!(CaptureMinUpdateInterval).and_then(|()| session.SetMinUpdateInterval(interval))
    {
        tracing::warn!(%error, "set minimum update interval failed");
    }
    border_hidden
}

fn request_borderless() {
    let status = GraphicsCaptureAccess::RequestAccessAsync(GraphicsCaptureAccessKind::Borderless)
        .and_then(|operation| operation.join());
    match status {
        Ok(AppCapabilityAccessStatus::Allowed) => {}
        Ok(status) => tracing::warn!(status = status.0, "borderless capture access not allowed"),
        Err(error) => tracing::warn!(%error, "borderless capture access request failed"),
    }
}
