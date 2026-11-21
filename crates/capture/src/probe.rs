use windows::Graphics::Capture::GraphicsCaptureSession;
use windows::Wdk::System::SystemServices::RtlGetVersion;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
use windows::Win32::Media::MediaFoundation::{
    IMFActivate, MFMediaType_Video, MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG_HARDWARE,
    MFT_ENUM_FLAG_SORTANDFILTER, MFT_ENUM_HARDWARE_VENDOR_ID_Attribute,
    MFT_FRIENDLY_NAME_Attribute, MFT_REGISTER_TYPE_INFO, MFTEnumEx, MFVideoFormat_H264,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::core::{GUID, PWSTR};

use crate::com::{ComGuard, MfGuard};
use crate::d3d::wide_to_string;

pub fn init_process() -> windows::core::Result<()> {
    // SAFETY: no pointers are passed; the call only changes this process's DPI awareness.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
}

pub fn system_report() -> String {
    let _com = ComGuard::mta().ok();
    let _mf = MfGuard::start().ok();
    let mut out = String::new();
    push_line(&mut out, format!("windows build: {}", windows_build()));
    push_line(
        &mut out,
        format!(
            "graphics capture supported: {}",
            GraphicsCaptureSession::IsSupported()
                .map_or_else(|e| format!("error ({e})"), |v| v.to_string())
        ),
    );
    push_adapters(&mut out);
    push_encoders(&mut out);
    out
}

fn push_line(out: &mut String, line: String) {
    out.push_str(&line);
    out.push('\n');
}

fn windows_build() -> String {
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is a live OSVERSIONINFOW with its size field set.
    let status = unsafe { RtlGetVersion(&mut info) };
    if status.is_err() {
        return format!("unknown (status {:#x})", status.0);
    }
    format!(
        "{}.{}.{}",
        info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
    )
}

fn push_adapters(out: &mut String) {
    // SAFETY: CreateDXGIFactory1 has no pointer arguments.
    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(factory) => factory,
        Err(e) => {
            push_line(out, format!("adapters: unavailable ({e})"));
            return;
        }
    };
    push_line(out, "adapters:".to_string());
    let mut index = 0;
    // SAFETY: enumeration only reads from live COM objects and writes return values.
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(index) } {
        // SAFETY: as above.
        match unsafe { adapter.GetDesc1() } {
            Ok(desc) => push_line(
                out,
                format!(
                    "  #{index} {} vendor={:#06x} device={:#06x} luid={:08x}:{:08x} vram={} MiB shared={} MiB",
                    wide_to_string(&desc.Description),
                    desc.VendorId,
                    desc.DeviceId,
                    desc.AdapterLuid.HighPart,
                    desc.AdapterLuid.LowPart,
                    desc.DedicatedVideoMemory / (1024 * 1024),
                    desc.SharedSystemMemory / (1024 * 1024),
                ),
            ),
            Err(e) => push_line(out, format!("  #{index} description unavailable ({e})")),
        }
        index += 1;
    }
}

fn push_encoders(out: &mut String) {
    match hardware_h264_encoders() {
        Ok(encoders) if encoders.is_empty() => {
            push_line(out, "hardware h264 encoders: none".to_string());
        }
        Ok(encoders) => {
            push_line(out, "hardware h264 encoders:".to_string());
            for (name, vendor) in encoders {
                push_line(out, format!("  {name} vendor={vendor}"));
            }
        }
        Err(e) => push_line(out, format!("hardware h264 encoders: error ({e})")),
    }
}

fn hardware_h264_encoders() -> windows::core::Result<Vec<(String, String)>> {
    let output_type = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };
    let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
    let mut count = 0u32;
    // SAFETY: the out pointers refer to live locals; `output_type` outlives the call.
    unsafe {
        MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
            None,
            Some(&output_type),
            &mut activates,
            &mut count,
        )?;
    }
    if activates.is_null() {
        return Ok(Vec::new());
    }
    // SAFETY: MFTEnumEx returned `count` initialised entries at `activates`.
    let entries = unsafe { std::slice::from_raw_parts_mut(activates, count as usize) };
    let encoders = entries
        .iter_mut()
        .filter_map(Option::take)
        .map(|activate| {
            (
                string_attribute(&activate, &MFT_FRIENDLY_NAME_Attribute),
                string_attribute(&activate, &MFT_ENUM_HARDWARE_VENDOR_ID_Attribute),
            )
        })
        .collect();
    // SAFETY: the array was allocated by MFTEnumEx with CoTaskMemAlloc and its interfaces are released.
    unsafe { CoTaskMemFree(Some(activates.cast_const().cast())) };
    Ok(encoders)
}

pub(crate) fn string_attribute(activate: &IMFActivate, key: &GUID) -> String {
    let mut value = PWSTR::null();
    let mut length = 0u32;
    // SAFETY: the out pointers refer to live locals.
    if unsafe { activate.GetAllocatedString(key, &mut value, &mut length) }.is_err()
        || value.is_null()
    {
        return "unknown".to_string();
    }
    // SAFETY: GetAllocatedString returned `length` valid UTF-16 units at `value`.
    let text =
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(value.0, length as usize) });
    // SAFETY: the string was allocated with CoTaskMemAlloc and is not used afterwards.
    unsafe { CoTaskMemFree(Some(value.0.cast_const().cast())) };
    text
}
