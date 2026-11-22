use windows::Win32::Media::MediaFoundation::{
    CLSID_MSH264EncoderMFT, CODECAPI_AVEncNumWorkerThreads, IMFTransform, MF_MT_AVG_BITRATE,
    MF_MT_DEFAULT_STRIDE, MF_MT_MPEG2_PROFILE, MFVideoFormat_H264, MFVideoFormat_NV12,
    eAVEncH264VProfile_High,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Variant::VARIANT;

use greyscreen_core::encoder;

use super::readback::Readback;
use super::setup::{
    Opened, Property, Spec, apply_codec_properties, apply_property, cap_warnings,
    read_sequence_header, start_streaming, stream_ids, video_type,
};
use super::transform::{Mft, RealMft};
use super::{Mode, Result};
use crate::d3d::GpuDevice;

const NAME: &str = "Microsoft H.264 Encoder MFT";
const QUALITY_VS_SPEED: u32 = 0;

pub(super) fn open(gpu: &GpuDevice, spec: &Spec) -> Result<Opened> {
    // SAFETY: the class id is a valid constant and no outer object is passed.
    let transform: IMFTransform =
        unsafe { CoCreateInstance(&CLSID_MSH264EncoderMFT, None, CLSCTX_INPROC_SERVER)? };
    let mft = RealMft::new(transform, None)?;
    let (mode, mismatches) = apply_codec_properties(&mft, spec, QUALITY_VS_SPEED)?;
    apply_property(&mft, &worker_threads())?;
    let (input_id, output_id) = stream_ids(&mft)?;
    let output_type = video_type(spec, &MFVideoFormat_H264)?;
    // SAFETY: the type is live and not modified after this point.
    unsafe {
        output_type.SetUINT32(&MF_MT_AVG_BITRATE, spec.mean_bitrate)?;
        output_type.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)?;
    }
    mft.set_output_type(output_id, &output_type)?;
    let input_type = video_type(spec, &MFVideoFormat_NV12)?;
    // SAFETY: the type is live and not modified after this point.
    unsafe { input_type.SetUINT32(&MF_MT_DEFAULT_STRIDE, spec.width)? };
    mft.set_input_type(input_id, &input_type)?;
    let sequence_header = read_sequence_header(&mft, output_id);
    start_streaming(&mft)?;
    let readback = Readback::new(gpu, spec.width, spec.height)?;
    tracing::info!(encoder = NAME, "software H.264 encoder configured");
    Ok(Opened {
        mode: Mode::Sync { readback },
        mft: Box::new(mft),
        input_id,
        output_id,
        name: format!("{NAME} ({})", mode.label()),
        sequence_header,
        warnings: cap_warnings(NAME, &mismatches),
    })
}

fn worker_threads() -> Property {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get() as u32);
    Property {
        name: "AVEncNumWorkerThreads",
        id: CODECAPI_AVEncNumWorkerThreads,
        value: VARIANT::from(encoder::worker_threads(cores)),
        required: false,
    }
}
