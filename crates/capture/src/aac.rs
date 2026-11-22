use std::mem::ManuallyDrop;

use windows::Win32::Media::MediaFoundation::{
    AACMFTEncoder, IMFMediaType, IMFSample, IMFTransform, MF_E_TRANSFORM_NEED_MORE_INPUT,
    MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION, MF_MT_AAC_PAYLOAD_TYPE,
    MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_BLOCK_ALIGNMENT,
    MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE,
    MF_MT_USER_DATA, MFAudioFormat_AAC, MFAudioFormat_PCM, MFCreateMediaType, MFCreateMemoryBuffer,
    MFCreateSample, MFMediaType_Audio, MFT_MESSAGE_COMMAND_DRAIN,
    MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, MFT_MESSAGE_NOTIFY_START_OF_STREAM, MFT_OUTPUT_DATA_BUFFER,
    MFT_OUTPUT_STREAM_PROVIDES_SAMPLES,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};

use crate::audio::PcmBlock;

const HNS_PER_SECOND: u128 = 10_000_000;
const BITS_PER_SAMPLE: u32 = 16;
const AAC_LC_PROFILE_LEVEL: u32 = 0x29;
const RAW_PAYLOAD: u32 = 0;
const HEAAC_WAVE_INFO_PREFIX: usize = 12;
const FALLBACK_OUTPUT_BYTES: u32 = 8192;

#[derive(Debug, thiserror::Error)]
pub enum AacError {
    #[error("media foundation: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("unsupported audio format: {0}")]
    Unsupported(&'static str),
}

pub type Result<T> = std::result::Result<T, AacError>;

pub trait PcmEncoder {
    fn encode(&mut self, block: &PcmBlock, out: &mut Vec<Vec<u8>>) -> Result<()>;
    fn finish(self: Box<Self>, out: &mut Vec<Vec<u8>>) -> Result<()>;
    fn asc(&self) -> Vec<u8>;
}

pub type OpenEncoder = Box<dyn FnOnce() -> Result<Box<dyn PcmEncoder>> + Send>;

pub fn open_aac(sample_rate: u32, channels: u16, bitrate: u32) -> OpenEncoder {
    Box::new(move || Ok(Box::new(AacEncoder::new(sample_rate, channels, bitrate)?)))
}

impl PcmEncoder for AacEncoder {
    fn encode(&mut self, block: &PcmBlock, out: &mut Vec<Vec<u8>>) -> Result<()> {
        AacEncoder::encode(self, block, out)
    }

    fn finish(self: Box<Self>, out: &mut Vec<Vec<u8>>) -> Result<()> {
        AacEncoder::finish(*self, out)
    }

    fn asc(&self) -> Vec<u8> {
        AacEncoder::asc(self)
    }
}

#[derive(Debug)]
pub struct AacEncoder {
    transform: IMFTransform,
    sample_rate: u32,
    channels: u16,
}

impl AacEncoder {
    pub fn new(sample_rate: u32, channels: u16, bitrate: u32) -> Result<Self> {
        if sample_rate == 0 || channels == 0 {
            return Err(AacError::Unsupported(
                "sample rate and channels must be nonzero",
            ));
        }
        // SAFETY: the CLSID is a valid constant and the result is a COM interface owned by the returned value.
        let transform: IMFTransform =
            unsafe { CoCreateInstance(&AACMFTEncoder, None, CLSCTX_INPROC_SERVER) }?;
        let output = output_type(sample_rate, channels, bitrate / 8)?;
        let input = input_type(sample_rate, channels)?;
        // SAFETY: both media types are live COM objects and stream id 0 is the only stream of this MFT.
        unsafe {
            transform.SetOutputType(0, &output, 0)?;
            transform.SetInputType(0, &input, 0)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)?;
        }
        Ok(Self {
            transform,
            sample_rate,
            channels,
        })
    }

    pub fn encode(&mut self, block: &PcmBlock, out: &mut Vec<Vec<u8>>) -> Result<()> {
        let frames = block.samples.len() / usize::from(self.channels);
        if frames == 0 {
            return Ok(());
        }
        let sample = self.input_sample(block, frames)?;
        // SAFETY: the sample is a live COM object and stream id 0 exists.
        unsafe { self.transform.ProcessInput(0, &sample, 0) }?;
        self.collect(out)
    }

    pub fn finish(self, out: &mut Vec<Vec<u8>>) -> Result<()> {
        // SAFETY: the transform is live and the drain message takes no parameter.
        unsafe { self.transform.ProcessMessage(MFT_MESSAGE_COMMAND_DRAIN, 0) }?;
        self.collect(out)
    }

    pub fn asc(&self) -> Vec<u8> {
        self.user_data_asc()
            .unwrap_or_else(|| greyscreen_mp4::aac::asc_lc(self.sample_rate, self.channels))
    }

    fn user_data_asc(&self) -> Option<Vec<u8>> {
        // SAFETY: the transform is live and stream id 0 exists.
        let current = unsafe { self.transform.GetOutputCurrentType(0) }.ok()?;
        // SAFETY: the media type is live and the blob size is queried before the buffer is sized.
        let size = unsafe { current.GetBlobSize(&MF_MT_USER_DATA) }.ok()? as usize;
        let mut blob = vec![0u8; size];
        // SAFETY: the buffer length equals the size reported by the media type.
        unsafe { current.GetBlob(&MF_MT_USER_DATA, &mut blob, None) }.ok()?;
        (blob.len() > HEAAC_WAVE_INFO_PREFIX).then(|| blob[HEAAC_WAVE_INFO_PREFIX..].to_vec())
    }

    fn input_sample(&self, block: &PcmBlock, frames: usize) -> Result<IMFSample> {
        let bytes = block.samples.len() * size_of::<i16>();
        let length = u32::try_from(bytes).map_err(|_| AacError::Unsupported("block too large"))?;
        let rate = u128::from(self.sample_rate);
        let time = u128::from(block.first_frame) * HNS_PER_SECOND / rate;
        let duration = (frames as u128 * HNS_PER_SECOND / rate).max(1);
        // SAFETY: the buffer is created with `length` bytes, locked, filled within that length and unlocked before use.
        unsafe {
            let buffer = MFCreateMemoryBuffer(length)?;
            let mut data = std::ptr::null_mut();
            buffer.Lock(&mut data, None, None)?;
            std::ptr::copy_nonoverlapping(block.samples.as_ptr().cast::<u8>(), data, bytes);
            buffer.Unlock()?;
            buffer.SetCurrentLength(length)?;
            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(time as i64)?;
            sample.SetSampleDuration(duration as i64)?;
            Ok(sample)
        }
    }

    fn collect(&self, out: &mut Vec<Vec<u8>>) -> Result<()> {
        // SAFETY: the transform is live and stream id 0 exists.
        let info = unsafe { self.transform.GetOutputStreamInfo(0) }?;
        let provides = info.dwFlags & MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32 != 0;
        let capacity = if info.cbSize == 0 {
            FALLBACK_OUTPUT_BYTES
        } else {
            info.cbSize
        };
        loop {
            let supplied = if provides {
                None
            } else {
                Some(output_sample(capacity)?)
            };
            let mut buffers = [MFT_OUTPUT_DATA_BUFFER {
                dwStreamID: 0,
                pSample: ManuallyDrop::new(supplied),
                dwStatus: 0,
                pEvents: ManuallyDrop::new(None),
            }];
            let mut status = 0u32;
            // SAFETY: `buffers` holds one initialized entry and `status` is a valid out pointer.
            let result = unsafe { self.transform.ProcessOutput(0, &mut buffers, &mut status) };
            let [buffer] = buffers;
            let sample = ManuallyDrop::into_inner(buffer.pSample);
            let _events = ManuallyDrop::into_inner(buffer.pEvents);
            match result {
                Ok(()) => {
                    if let Some(sample) = sample {
                        out.push(read_sample(&sample)?);
                    }
                }
                Err(error) if error.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }
}

fn output_sample(capacity: u32) -> Result<IMFSample> {
    // SAFETY: plain MF constructors; the buffer is owned by the sample returned.
    unsafe {
        let buffer = MFCreateMemoryBuffer(capacity)?;
        let sample = MFCreateSample()?;
        sample.AddBuffer(&buffer)?;
        Ok(sample)
    }
}

fn read_sample(sample: &IMFSample) -> Result<Vec<u8>> {
    // SAFETY: the buffer is locked for the copy, the copy is bounded by the reported current length, and it is unlocked after.
    unsafe {
        let buffer = sample.ConvertToContiguousBuffer()?;
        let mut data = std::ptr::null_mut();
        let mut length = 0u32;
        buffer.Lock(&mut data, None, Some(&mut length))?;
        let bytes = std::slice::from_raw_parts(data, length as usize).to_vec();
        buffer.Unlock()?;
        Ok(bytes)
    }
}

fn output_type(sample_rate: u32, channels: u16, bytes_per_second: u32) -> Result<IMFMediaType> {
    let media = audio_type(&MFAudioFormat_AAC, sample_rate, channels)?;
    // SAFETY: the media type is a live COM object and every key is a valid constant.
    unsafe {
        media.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, bytes_per_second)?;
        media.SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, RAW_PAYLOAD)?;
        media.SetUINT32(
            &MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION,
            AAC_LC_PROFILE_LEVEL,
        )?;
    }
    Ok(media)
}

fn input_type(sample_rate: u32, channels: u16) -> Result<IMFMediaType> {
    let media = audio_type(&MFAudioFormat_PCM, sample_rate, channels)?;
    let align = u32::from(channels) * BITS_PER_SAMPLE / 8;
    // SAFETY: the media type is a live COM object and every key is a valid constant.
    unsafe {
        media.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, align)?;
        media.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, align * sample_rate)?;
    }
    Ok(media)
}

fn audio_type(
    subtype: &windows::core::GUID,
    sample_rate: u32,
    channels: u16,
) -> Result<IMFMediaType> {
    // SAFETY: the media type is created here and every key is a valid constant.
    unsafe {
        let media = MFCreateMediaType()?;
        media.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
        media.SetGUID(&MF_MT_SUBTYPE, subtype)?;
        media.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, BITS_PER_SAMPLE)?;
        media.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, sample_rate)?;
        media.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, u32::from(channels))?;
        Ok(media)
    }
}
