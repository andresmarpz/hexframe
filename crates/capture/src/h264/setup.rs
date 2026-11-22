use super::open_error::{self, Step};
use super::rate::{self, RateMode};
use windows::Win32::Foundation::{E_NOTIMPL, E_POINTER, LUID, S_OK};
use windows::Win32::Media::MediaFoundation::{
    CODECAPI_AVEncCommonQualityVsSpeed, CODECAPI_AVEncH264CABACEnable,
    CODECAPI_AVEncMPVDefaultBPictureCount, CODECAPI_AVEncMPVGOPSize, CODECAPI_AVLowLatencyMode,
    ICodecAPI, IMFActivate, IMFAttributes, IMFDXGIDeviceManager, IMFMediaEventGenerator,
    IMFMediaType, IMFTransform, MF_LOW_LATENCY, MF_MT_AVG_BITRATE, MF_MT_FRAME_RATE,
    MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE, MF_MT_MPEG_SEQUENCE_HEADER,
    MF_MT_MPEG2_PROFILE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE, MF_SA_D3D11_AWARE,
    MF_SA_D3D11_BINDFLAGS, MF_TRANSFORM_ASYNC, MF_TRANSFORM_ASYNC_UNLOCK, MFCreateAttributes,
    MFCreateDXGIDeviceManager, MFCreateMediaType, MFMediaType_Video, MFT_CATEGORY_VIDEO_ENCODER,
    MFT_ENUM_ADAPTER_LUID, MFT_ENUM_FLAG_HARDWARE, MFT_ENUM_FLAG_SORTANDFILTER,
    MFT_ENUM_HARDWARE_VENDOR_ID_Attribute, MFT_FRIENDLY_NAME_Attribute, MFT_MESSAGE_COMMAND_FLUSH,
    MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, MFT_MESSAGE_NOTIFY_START_OF_STREAM,
    MFT_MESSAGE_SET_D3D_MANAGER, MFT_REGISTER_TYPE_INFO, MFTEnum2, MFTEnumEx, MFVideoFormat_H264,
    MFVideoFormat_NV12, MFVideoInterlace_Progressive, eAVEncH264VProfile_High,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Variant::VARIANT;
use windows::core::{GUID, Interface};

use super::{H264Error, Mode, Result};
use crate::d3d::GpuDevice;
use crate::probe::string_attribute;

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
    pub(super) transform: IMFTransform,
    pub(super) codec: ICodecAPI,
    pub(super) input_id: u32,
    pub(super) output_id: u32,
    pub(super) name: String,
    pub(super) sequence_header: Vec<u8>,
}

struct Configured {
    transform: IMFTransform,
    events: IMFMediaEventGenerator,
    codec: ICodecAPI,
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
    if candidates.is_empty() {
        return Err(H264Error::NoHardwareEncoder(
            "enumeration returned no hardware H.264 encoder for this adapter".to_string(),
        ));
    }
    let mut failures = Vec::new();
    for activate in candidates {
        let name = describe(&activate);
        match configure(&activate, &manager, spec) {
            Ok(configured) => {
                tracing::info!(encoder = %name, "hardware H.264 encoder configured");
                return Ok(Opened {
                    mode: Mode::Async {
                        events: configured.events,
                        activate,
                        _manager: manager,
                    },
                    transform: configured.transform,
                    codec: configured.codec,
                    input_id: configured.input_id,
                    output_id: configured.output_id,
                    name: format!("{name} ({})", configured.mode.label()),
                    sequence_header: configured.sequence_header,
                });
            }
            Err(error) => {
                tracing::warn!(encoder = %name, %error, "hardware H.264 encoder candidate rejected");
                failures.push(format!("{name}: {error}"));
                // SAFETY: the activate object is live; shutting it down releases the failed transform.
                if let Err(error) = unsafe { activate.ShutdownObject() } {
                    tracing::debug!(%error, "candidate shutdown failed");
                }
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

fn enumerate(luid: LUID) -> Result<Vec<IMFActivate>> {
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
        Ok(()) => Ok(take_activates(activates, count)),
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
            Ok(all)
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

fn describe(activate: &IMFActivate) -> String {
    format!(
        "{} (vendor {})",
        string_attribute(activate, &MFT_FRIENDLY_NAME_Attribute),
        string_attribute(activate, &MFT_ENUM_HARDWARE_VENDOR_ID_Attribute)
    )
}

fn rejected(reason: &str) -> H264Error {
    open_error::unsupported(reason)
}

fn configure(
    activate: &IMFActivate,
    manager: &IMFDXGIDeviceManager,
    spec: &Spec,
) -> Result<Configured> {
    fault!(MftActivation)?;
    // SAFETY: the activate object is live.
    let transform: IMFTransform =
        unsafe { activate.ActivateObject() }.map_err(|e| open_error::at(Step::Activate, &e))?;
    // SAFETY: the transform is live.
    let attributes = unsafe { transform.GetAttributes()? };
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
    // SAFETY: the transform and manager are live; the pointer stays valid for the call.
    let device_set =
        unsafe { transform.ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, manager.as_raw() as usize) };
    if let Err(error) = device_set {
        return Err(rejected(&format!("SET_D3D_MANAGER failed: {error}")));
    }
    let (input_id, output_id) = stream_ids(&transform)?;
    let codec: ICodecAPI = transform.cast()?;
    let mode = apply_codec_properties(&codec, spec, HARDWARE_QUALITY_VS_SPEED)?;
    let output_type = video_type(spec, &MFVideoFormat_H264)?;
    // SAFETY: the type is live and not modified after this point.
    unsafe {
        output_type.SetUINT32(&MF_MT_AVG_BITRATE, spec.mean_bitrate)?;
        output_type.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)?;
    }
    // SAFETY: the transform and type are live.
    unsafe { transform.SetOutputType(output_id, &output_type, 0) }
        .map_err(|e| open_error::at(Step::OutputType, &e))?;
    let mut sequence_header = read_sequence_header(&transform, output_id);
    log_input_types(&transform, input_id);
    let input_type = video_type(spec, &MFVideoFormat_NV12)?;
    // SAFETY: the transform and type are live.
    if let Err(error) = unsafe { transform.SetInputType(input_id, &input_type, 0) } {
        return Err(open_error::at(Step::InputType, &error));
    }
    if sequence_header.is_empty() {
        sequence_header = read_sequence_header(&transform, output_id);
    }
    if sequence_header.is_empty() {
        tracing::warn!("encoder did not publish MF_MT_MPEG_SEQUENCE_HEADER");
    }
    log_bind_flags(&transform, input_id);
    let events: IMFMediaEventGenerator = transform.cast()?;
    start_streaming(&transform).map_err(|error| match error {
        H264Error::Windows(e) => open_error::at(Step::Start, &e),
        other => other,
    })?;
    Ok(Configured {
        transform,
        events,
        codec,
        input_id,
        output_id,
        sequence_header,
        mode,
    })
}

pub(super) fn stream_ids(transform: &IMFTransform) -> Result<(u32, u32)> {
    let mut inputs = [0u32; 1];
    let mut outputs = [0u32; 1];
    // SAFETY: both slices are live and hold one id each.
    match unsafe { transform.GetStreamIDs(&mut inputs, &mut outputs) } {
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

pub(super) fn read_sequence_header(transform: &IMFTransform, output_id: u32) -> Vec<u8> {
    // SAFETY: the transform is live.
    let Ok(current) = (unsafe { transform.GetOutputCurrentType(output_id) }) else {
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

fn log_input_types(transform: &IMFTransform, input_id: u32) {
    for index in 0..MAX_LISTED_TYPES {
        // SAFETY: the transform is live.
        let Ok(available) = (unsafe { transform.GetInputAvailableType(input_id, index) }) else {
            break;
        };
        // SAFETY: the type is live.
        let subtype = unsafe { available.GetGUID(&MF_MT_SUBTYPE) }.ok();
        tracing::info!(index, ?subtype, "encoder input type available");
    }
}

fn log_bind_flags(transform: &IMFTransform, input_id: u32) {
    // SAFETY: the transform is live.
    let Ok(attributes) = (unsafe { transform.GetInputStreamAttributes(input_id) }) else {
        return;
    };
    // SAFETY: the attribute store is live.
    if let Ok(flags) = unsafe { attributes.GetUINT32(&MF_SA_D3D11_BINDFLAGS) } {
        tracing::info!(flags, "encoder input bind flags");
    }
}

pub(super) fn start_streaming(transform: &IMFTransform) -> Result<()> {
    for message in [
        MFT_MESSAGE_COMMAND_FLUSH,
        MFT_MESSAGE_NOTIFY_BEGIN_STREAMING,
        MFT_MESSAGE_NOTIFY_START_OF_STREAM,
    ] {
        // SAFETY: the transform is live and the messages carry no parameter.
        unsafe { transform.ProcessMessage(message, 0)? };
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
    codec: &ICodecAPI,
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

pub(super) fn apply_property(codec: &ICodecAPI, property: &Property) -> Result<Applied> {
    // SAFETY: the vtable call receives the live interface pointer and a GUID that outlives it.
    let supported =
        unsafe { (Interface::vtable(codec).IsSupported)(Interface::as_raw(codec), &property.id) };
    // SAFETY: the codec interface is live and the value outlives the call.
    let set = unsafe { codec.SetValue(&property.id, &property.value) };
    // SAFETY: the codec interface is live.
    let read_back = unsafe { codec.GetValue(&property.id) };
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
