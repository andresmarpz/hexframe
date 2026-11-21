use std::mem::size_of;

use greyscreen_core::crop::CropRect;
use greyscreen_core::health::CaptureSetup;
use greyscreen_core::media::{CaptureConfig, CaptureTarget};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplaySettingsW, GetMonitorInfoW, HMONITOR,
    MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MONITORINFOEXW,
    MonitorFromWindow,
};
use windows::core::PCWSTR;

use crate::d3d::GpuDevice;

#[derive(Debug, Clone, Copy, Default)]
struct DisplayMode {
    width: u32,
    height: u32,
    hz: u32,
}

pub(super) struct Inputs<'a> {
    pub(super) target: &'a CaptureTarget,
    pub(super) cfg: &'a CaptureConfig,
    pub(super) gpu: &'a GpuDevice,
    pub(super) frame: (u32, u32),
    pub(super) crop: CropRect,
    pub(super) window: Option<(u32, u32)>,
    pub(super) encode: (u32, u32),
    pub(super) encoder: String,
}

pub(super) fn describe(inputs: Inputs) -> CaptureSetup {
    let display = display_mode(inputs.target).unwrap_or_default();
    let window = inputs.window.unwrap_or_default();
    CaptureSetup {
        adapter: inputs.gpu.adapter_name.clone(),
        driver_version: inputs.gpu.driver_version.clone(),
        display_width: display.width,
        display_height: display.height,
        display_hz: display.hz,
        window_width: window.0,
        window_height: window.1,
        frame_width: inputs.frame.0,
        frame_height: inputs.frame.1,
        crop_left: inputs.crop.left,
        crop_top: inputs.crop.top,
        crop_width: inputs.crop.width,
        crop_height: inputs.crop.height,
        encode_width: inputs.encode.0,
        encode_height: inputs.encode.1,
        fps: inputs.cfg.fps,
        encoder: inputs.encoder,
        bitrate: inputs.cfg.video_bitrate,
        peak: inputs.cfg.video_peak,
        quality: inputs.cfg.quality,
        gop_frames: inputs.cfg.gop_frames,
        audio: inputs.cfg.audio,
    }
}

fn monitor_for(target: &CaptureTarget) -> HMONITOR {
    match *target {
        CaptureTarget::GameWindow { hwnd, .. } => {
            let hwnd = HWND(hwnd as *mut core::ffi::c_void);
            // SAFETY: MonitorFromWindow accepts any handle value and falls back to the nearest monitor.
            unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }
        }
        CaptureTarget::PrimaryMonitor => {
            // SAFETY: a null window handle with DEFAULTTOPRIMARY returns the primary monitor.
            unsafe { MonitorFromWindow(HWND::default(), MONITOR_DEFAULTTOPRIMARY) }
        }
    }
}

fn display_mode(target: &CaptureTarget) -> Option<DisplayMode> {
    let monitor = monitor_for(target);
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    let base: *mut MONITORINFO = (&raw mut info).cast();
    // SAFETY: `info` is a live MONITORINFOEXW with cbSize set, and the base pointer addresses its first field.
    if !unsafe { GetMonitorInfoW(monitor, base) }.as_bool() {
        return None;
    }
    let mut mode = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    // SAFETY: `szDevice` is a nul-terminated name inside the live `info`, and `mode` has dmSize set.
    let read = unsafe {
        EnumDisplaySettingsW(
            PCWSTR(info.szDevice.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut mode,
        )
    };
    read.as_bool().then_some(DisplayMode {
        width: mode.dmPelsWidth,
        height: mode.dmPelsHeight,
        hz: mode.dmDisplayFrequency,
    })
}
