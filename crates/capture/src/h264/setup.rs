use greyscreen_core::encoder::OpenStep;

use super::open_error;
use super::rate::{self, RateMode};
use windows::Win32::Foundation::{E_NOTIMPL, E_POINTER, LUID, S_OK};
use windows::Win32::Media::MediaFoundation::{
    CODECAPI_AVEncCommonQualityVsSpeed, CODECAPI_AVEncH264CABACEnable,
    CODECAPI_AVEncMPVDefaultBPictureCount, CODECAPI_AVEncMPVGOPSize, CODECAPI_AVLowLatencyMode,
    IMFActivate, IMFAttributes, IMFDXGIDeviceManager, IMFMediaType, MF_LOW_LATENCY,
    MF_MT_AVG_BITRATE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE,
    MF_MT_MPEG_SEQUENCE_HEADER, MF_MT_MPEG2_PROFILE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE,
    MF_SA_D3D11_AWARE, MF_SA_D3D11_BINDFLAGS, MF_TRANSFORM_ASYNC, MF_TRANSFORM_ASYNC_UNLOCK,
    MFCreateAttributes, MFCreateDXGIDeviceManager, MFCreateMediaType, MFMediaType_Video,
    MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_ADAPTER_LUID, MFT_ENUM_FLAG_HARDWARE,
    MFT_ENUM_FLAG_SORTANDFILTER, MFT_MESSAGE_COMMAND_FLUSH, MFT_MESSAGE_NOTIFY_BEGIN_STREAMING,
    MFT_MESSAGE_NOTIFY_START_OF_STREAM, MFT_REGISTER_TYPE_INFO, MFTEnum2, MFTEnumEx,
    MFVideoFormat_H264, MFVideoFormat_NV12, MFVideoInterlace_Progressive, eAVEncH264VProfile_High,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Variant::VARIANT;
use windows::core::GUID;

use super::transform::{Candidate, Mft, RealCandidate};
use super::{H264Error, Mode, Result};
use crate::d3d::GpuDevice;

const MAX_LISTED_TYPES: u32 = 64;
const HARDWARE_QUALITY_VS_SPEED: u32 = 100;

#[derive(Debug, Clone, Copy)]
pub(super) struct Spec {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) fps: u32,
    pub(super) mean_bitrate: u32,
    pub(super) peak_bitrate: u32,
    pub(super) quality: Option<u32>,
    pub(super) gop_frames: u32,
}

#[derive(Debug)]
pub(super) struct Opened {
    pub(super) mode: Mode,
    pub(super) mft: Box<dyn Mft>,
    pub(super) input_id: u32,
    pub(super) output_id: u32,
    pub(super) name: String,
    pub(super) sequence_header: Vec<u8>,
}

struct Configured {
    mft: Box<dyn Mft>,
    input_id: u32,
    output_id: u32,
    sequence_header: Vec<u8>,
    mode: RateMode,
}

pub(super) struct Property {
    pub(super) name: &'static str,
    pub(super) id: GUID,
    pub(super) value: VARIANT,
    pub(super) required: bool,
}

pub(super) fn open(gpu: &GpuDevice, spec: &Spec) -> Result<Opened> {
    let manager = create_device_manager(gpu)?;
    let candidates = enumerate(gpu.adapter_luid)?;
    open_candidates(candidates, manager, spec, gpu.vendor_id)
}

pub(super) fn open_candidates(
    candidates: Vec<Box<dyn Candidate>>,
    manager: IMFDXGIDeviceManager,
    spec: &Spec,
    vendor_id: u32,
) -> Result<Opened> {
    if candidates.is_empty() {
        return Err(H264Error::NoHardwareEncoder(
            "enumeration returned no hardware H.264 encoder for this adapter".to_string(),
        ));
    }
    let mut failures = Vec::new();
    for candidate in candidates {
        let name = candidate.describe();
        match configure(candidate.as_ref(), &manager, spec, vendor_id) {
            Ok(configured) => {
                tracing::info!(encoder = %name, "hardware H.264 encoder configured");
                return Ok(Opened {
                    mode: Mode::Async { _manager: manager },
                    mft: configured.mft,
                    input_id: configured.input_id,
                    output_id: configured.output_id,
                    name: format!("{name} ({})", configured.mode.label()),
                    sequence_header: configured.sequence_header,
                });
            }
            Err(error) => {
                let error = open_error::tag_untagged(error, vendor_id);
                tracing::warn!(encoder = %name, %error, "hardware H.264 encoder candidate rejected");
                failures.push(format!("{name}: {error}"));
                candidate.shutdown_object();
            }
        }
    }
    Err(H264Error::NoHardwareEncoder(failures.join("; ")))
}

fn create_device_manager(gpu: &GpuDevice) -> Result<IMFDXGIDeviceManager> {
    let mut token = 0u32;
    let mut manager = None;
    // SAFETY: the out pointers refer to live locals.
    unsafe { MFCreateDXGIDeviceManager(&mut token, &mut manager)? };
    let manager = manager.ok_or_else(|| windows::core::Error::from(E_POINTER))?;
    // SAFETY: the manager and device are live.
    unsafe { manager.ResetDevice(&gpu.device, token)? };
    Ok(manager)
}

fn luid_bytes(luid: LUID) -> [u8; 8] {
    let mut bytes = [0u8; 8];
    bytes[..4].copy_from_slice(&luid.LowPart.to_le_bytes());
    bytes[4..].copy_from_slice(&luid.HighPart.to_le_bytes());
    bytes
}

fn enumerate(luid: LUID) -> Result<Vec<Box<dyn Candidate>>> {
    let output_type = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };
    let flags = MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER;
    let mut attributes = None;
    // SAFETY: the out pointer refers to a live local.
    unsafe { MFCreateAttributes(&mut attributes, 1)? };
    let attributes: IMFAttributes =
        attributes.ok_or_else(|| windows::core::Error::from(E_POINTER))?;
    // SAFETY: the attribute store is live and the blob is copied.
    unsafe { attributes.SetBlob(&MFT_ENUM_ADAPTER_LUID, &luid_bytes(luid))? };
    let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
    let mut count = 0u32;
    // SAFETY: the out pointers refer to live locals; the type and attributes outlive the call.
    let result = unsafe {
        MFTEnum2(
            MFT_CATEGORY_VIDEO_ENCODER,
            flags,
            None,
            Some(&output_type),
            &attributes,
            &mut activates,
            &mut count,
        )
    };
    match result {
        Ok(()) => Ok(boxed(take_activates(activates, count))),
        Err(error) => {
            tracing::info!(%error, "MFTEnum2 failed, falling back to MFTEnumEx with an adapter filter");
            let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
            let mut count = 0u32;
            // SAFETY: the out pointers refer to live locals; `output_type` outlives the call.
            unsafe {
                MFTEnumEx(
                    MFT_CATEGORY_VIDEO_ENCODER,
                    flags,
                    None,
                    Some(&output_type),
                    &mut activates,
                    &mut count,
                )?;
            }
            let mut all = take_activates(activates, count);
            all.retain(|activate| matches_luid(activate, luid));
            Ok(boxed(all))
        }
    }
}

fn matches_luid(activate: &IMFActivate, luid: LUID) -> bool {
    let mut blob = [0u8; 8];
    // SAFETY: `blob` is a live buffer of the size passed.
    match unsafe { activate.GetBlob(&MFT_ENUM_ADAPTER_LUID, &mut blob, None) } {
        Ok(()) => blob == luid_bytes(luid),
        Err(_) => true,
    }
}

fn take_activates(activates: *mut Option<IMFActivate>, count: u32) -> Vec<IMFActivate> {
    if activates.is_null() {
        return Vec::new();
    }
    // SAFETY: the enumeration returned `count` initialised entries at `activates`.
    let entries = unsafe { std::slice::from_raw_parts_mut(activates, count as usize) };
    let taken = entries.iter_mut().filter_map(Option::take).collect();
    // SAFETY: the array came from CoTaskMemAlloc and every interface in it was taken.
    unsafe { CoTaskMemFree(Some(activates.cast_const().cast())) };
    taken
}

fn boxed(activates: Vec<IMFActivate>) -> Vec<Box<dyn Candidate>> {
    activates
        .into_iter()
        .map(|activate| Box::new(RealCandidate(activate)) as Box<dyn Candidate>)
        .collect()
}

fn rejected(reason: &str) -> H264Error {
    open_error::unsupported(reason)
}

fn configure(
    candidate: &dyn Candidate,
    manager: &IMFDXGIDeviceManager,
    spec: &Spec,
    vendor_id: u32,
) -> Result<Configured> {
    fault!(MftActivation)?;
    let mft = candidate
        .activate()
        .map_err(|e| open_error::at(OpenStep::Activate, vendor_id, &e))?;
    let attributes = mft.attributes()?;
    // SAFETY: the attribute store is live.
    let is_async = unsafe { attributes.GetUINT32(&MF_TRANSFORM_ASYNC) }.unwrap_or(0);
    if is_async != 1 {
        return Err(rejected("the encoder is not an asynchronous MFT"));
    }
    // SAFETY: the attribute store is live.
    unsafe { attributes.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1)? };
    // SAFETY: the attribute store is live.
    if let Err(error) = unsafe { attributes.SetUINT32(&MF_LOW_LATENCY, 1) } {
        tracing::info!(%error, "MF_LOW_LATENCY attribute not accepted");
    }
    // SAFETY: the attribute store is live.
    let d3d_aware = unsafe { attributes.GetUINT32(&MF_SA_D3D11_AWARE) }.unwrap_or(0);
    if d3d_aware != 1 {
        return Err(rejected("the encoder is not D3D11 aware"));
    }
    if let Err(error) = mft.set_d3d_manager(manager) {
        return Err(open_error::at(OpenStep::D3dManager, vendor_id, &error));
    }
    let (input_id, output_id) = stream_ids(&*mft)?;
    let mode = apply_codec_properties(&*mft, spec, HARDWARE_QUALITY_VS_SPEED)?;
    let output_type = video_type(spec, &MFVideoFormat_H264)?;
    // SAFETY: the type is live and not modified after this point.
    unsafe {
        output_type.SetUINT32(&MF_MT_AVG_BITRATE, spec.mean_bitrate)?;
        output_type.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)?;
    }
    mft.set_output_type(output_id, &output_type)
        .map_err(|e| open_error::at(OpenStep::OutputType, vendor_id, &e))?;
    let mut sequence_header = read_sequence_header(&*mft, output_id);
    log_input_types(&*mft, input_id);
    let input_type = video_type(spec, &MFVideoFormat_NV12)?;
    if let Err(error) = mft.set_input_type(input_id, &input_type) {
        return Err(open_error::at(OpenStep::InputType, vendor_id, &error));
    }
    if sequence_header.is_empty() {
        sequence_header = read_sequence_header(&*mft, output_id);
    }
    if sequence_header.is_empty() {
        tracing::warn!("encoder did not publish MF_MT_MPEG_SEQUENCE_HEADER");
    }
    log_bind_flags(&*mft, input_id);
    start_streaming(&*mft).map_err(|error| match error {
        H264Error::Windows(e) => open_error::at(OpenStep::Start, vendor_id, &e),
        other => other,
    })?;
    Ok(Configured {
        mft,
        input_id,
        output_id,
        sequence_header,
        mode,
    })
}

pub(super) fn stream_ids(mft: &dyn Mft) -> Result<(u32, u32)> {
    let mut inputs = [0u32; 1];
    let mut outputs = [0u32; 1];
    match mft.stream_ids(&mut inputs, &mut outputs) {
        Ok(()) => Ok((inputs[0], outputs[0])),
        Err(error) if error.code() == E_NOTIMPL => Ok((0, 0)),
        Err(error) => Err(error.into()),
    }
}

fn pack_u32_pair(high: u32, low: u32) -> u64 {
    (u64::from(high) << 32) | u64::from(low)
}

pub(super) fn video_type(spec: &Spec, subtype: &GUID) -> Result<IMFMediaType> {
    // SAFETY: MFCreateMediaType has no pointer arguments.
    let media_type = unsafe { MFCreateMediaType()? };
    // SAFETY: the type is live and owned by this call; the GUIDs outlive each call.
    unsafe {
        media_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        media_type.SetGUID(&MF_MT_SUBTYPE, subtype)?;
        media_type.SetUINT64(&MF_MT_FRAME_SIZE, pack_u32_pair(spec.width, spec.height))?;
        media_type.SetUINT64(&MF_MT_FRAME_RATE, pack_u32_pair(spec.fps, 1))?;
        media_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack_u32_pair(1, 1))?;
        media_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
    }
    super::colour::apply(&media_type)?;
    Ok(media_type)
}

pub(super) fn read_sequence_header(mft: &dyn Mft, output_id: u32) -> Vec<u8> {
    let Ok(current) = mft.output_current_type(output_id) else {
        return Vec::new();
    };
    // SAFETY: the type is live.
    let Ok(size) = (unsafe { current.GetBlobSize(&MF_MT_MPEG_SEQUENCE_HEADER) }) else {
        return Vec::new();
    };
    let mut header = vec![0u8; size as usize];
    // SAFETY: `header` has exactly `size` bytes.
    match unsafe { current.GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut header, None) } {
        Ok(()) => header,
        Err(_) => Vec::new(),
    }
}

fn log_input_types(mft: &dyn Mft, input_id: u32) {
    for index in 0..MAX_LISTED_TYPES {
        let Ok(available) = mft.input_available_type(input_id, index) else {
            break;
        };
        // SAFETY: the type is live.
        let subtype = unsafe { available.GetGUID(&MF_MT_SUBTYPE) }.ok();
        tracing::info!(index, ?subtype, "encoder input type available");
    }
}

fn log_bind_flags(mft: &dyn Mft, input_id: u32) {
    let Ok(attributes) = mft.input_stream_attributes(input_id) else {
        return;
    };
    // SAFETY: the attribute store is live.
    if let Ok(flags) = unsafe { attributes.GetUINT32(&MF_SA_D3D11_BINDFLAGS) } {
        tracing::info!(flags, "encoder input bind flags");
    }
}

pub(super) fn start_streaming(mft: &dyn Mft) -> Result<()> {
    for message in [
        MFT_MESSAGE_COMMAND_FLUSH,
        MFT_MESSAGE_NOTIFY_BEGIN_STREAMING,
        MFT_MESSAGE_NOTIFY_START_OF_STREAM,
    ] {
        mft.process_message(message)?;
    }
    Ok(())
}

fn base_properties(spec: &Spec, quality_vs_speed: u32) -> Vec<Property> {
    vec![
        Property {
            name: "AVEncMPVGOPSize",
            id: CODECAPI_AVEncMPVGOPSize,
            value: VARIANT::from(spec.gop_frames),
            required: true,
        },
        Property {
            name: "AVEncMPVDefaultBPictureCount",
            id: CODECAPI_AVEncMPVDefaultBPictureCount,
            value: VARIANT::from(0u32),
            required: false,
        },
        Property {
            name: "AVLowLatencyMode",
            id: CODECAPI_AVLowLatencyMode,
            value: VARIANT::from(true),
            required: false,
        },
        Property {
            name: "AVEncH264CABACEnable",
            id: CODECAPI_AVEncH264CABACEnable,
            value: VARIANT::from(true),
            required: false,
        },
        Property {
            name: "AVEncCommonQualityVsSpeed",
            id: CODECAPI_AVEncCommonQualityVsSpeed,
            value: VARIANT::from(quality_vs_speed),
            required: false,
        },
    ]
}

pub(super) fn apply_codec_properties(
    codec: &dyn Mft,
    spec: &Spec,
    quality_vs_speed: u32,
) -> Result<RateMode> {
    let mode = rate::apply(codec, spec)?;
    for property in base_properties(spec, quality_vs_speed) {
        apply_property(codec, &property)?;
    }
    Ok(mode)
}

pub(super) struct Applied {
    pub(super) set: bool,
    pub(super) read_back: Option<u32>,
}

pub(super) fn apply_property(codec: &dyn Mft, property: &Property) -> Result<Applied> {
    let supported = codec.is_supported(&property.id);
    let set = codec.set_value(&property.id, &property.value);
    let read_back = codec.get_value(&property.id);
    tracing::info!(
        property = property.name,
        supported = supported == S_OK,
        supported_code = format_args!("{:#010x}", supported.0),
        set = ?set.as_ref().map_err(|e| e.code()),
        read_back = %read_back.as_ref().map_or_else(|e| format!("error {:#010x}", e.code().0), describe_variant),
        "codec property"
    );
    let applied = Applied {
        set: set.is_ok(),
        read_back: read_back.ok().and_then(|value| u32::try_from(&value).ok()),
    };
    match set {
        Err(source) if property.required => Err(H264Error::Property {
            property: property.name,
            source,
        }),
        _ => Ok(applied),
    }
}

fn describe_variant(value: &VARIANT) -> String {
    if let Ok(number) = u32::try_from(value) {
        return number.to_string();
    }
    if let Ok(number) = i32::try_from(value) {
        return number.to_string();
    }
    if let Ok(flag) = bool::try_from(value) {
        return flag.to_string();
    }
    "unreadable variant type".to_string()
}
