use windows::Win32::Media::MediaFoundation::{
    CLSID_MSH264EncoderMFT, CODECAPI_AVEncNumWorkerThreads, ICodecAPI, IMFTransform,
    MF_MT_AVG_BITRATE, MF_MT_DEFAULT_STRIDE, MF_MT_MPEG2_PROFILE, MFVideoFormat_H264,
    MFVideoFormat_NV12, eAVEncH264VProfile_High,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Variant::VARIANT;
use windows::core::Interface;

use super::readback::Readback;
use super::setup::{
    Opened, Property, Spec, apply_codec_properties, apply_property, read_sequence_header,
    start_streaming, stream_ids, video_type,
};
use super::{Mode, Result};
use crate::d3d::GpuDevice;

const NAME: &str = "Microsoft H.264 Encoder MFT";
const QUALITY_VS_SPEED: u32 = 0;
const MAX_WORKER_THREADS: u32 = 4;
const MIN_WORKER_THREADS: u32 = 2;

pub(super) fn open(gpu: &GpuDevice, spec: &Spec) -> Result<Opened> {
    // SAFETY: the class id is a valid constant and no outer object is passed.
    let transform: IMFTransform =
        unsafe { CoCreateInstance(&CLSID_MSH264EncoderMFT, None, CLSCTX_INPROC_SERVER)? };
    let codec: ICodecAPI = transform.cast()?;
    let mode = apply_codec_properties(&codec, spec, QUALITY_VS_SPEED)?;
    apply_property(&codec, &worker_threads())?;
    let (input_id, output_id) = stream_ids(&transform)?;
    let output_type = video_type(spec, &MFVideoFormat_H264)?;
    // SAFETY: the type is live and not modified after this point.
    unsafe {
        output_type.SetUINT32(&MF_MT_AVG_BITRATE, spec.mean_bitrate)?;
        output_type.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)?;
    }
    // SAFETY: the transform and type are live.
    unsafe { transform.SetOutputType(output_id, &output_type, 0)? };
    let input_type = video_type(spec, &MFVideoFormat_NV12)?;
    // SAFETY: the type is live and not modified after this point.
    unsafe { input_type.SetUINT32(&MF_MT_DEFAULT_STRIDE, spec.width)? };
    // SAFETY: the transform and type are live.
    unsafe { transform.SetInputType(input_id, &input_type, 0)? };
    let sequence_header = read_sequence_header(&transform, output_id);
    start_streaming(&transform)?;
    let readback = Readback::new(gpu, spec.width, spec.height)?;
    tracing::info!(encoder = NAME, "software H.264 encoder configured");
    Ok(Opened {
        mode: Mode::Sync { readback },
        transform,
        codec,
        input_id,
        output_id,
        name: format!("{NAME} ({})", mode.label()),
        sequence_header,
    })
}

fn worker_threads() -> Property {
    let cores = std::thread::available_parallelism().map_or(MIN_WORKER_THREADS, |n| n.get() as u32);
    Property {
        name: "AVEncNumWorkerThreads",
        id: CODECAPI_AVEncNumWorkerThreads,
        value: VARIANT::from((cores / 2).clamp(MIN_WORKER_THREADS, MAX_WORKER_THREADS)),
        required: false,
    }
}
