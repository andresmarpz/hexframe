use std::path::Path;

use greyscreen_core::export::EncodePlan;
use windows::Win32::Media::MediaFoundation::{
    IMFAttributes, IMFByteStream, IMFMediaType, IMFSample, IMFSinkWriter,
    MF_MPEG4SINK_MOOV_BEFORE_MDAT, MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION,
    MF_MT_AAC_PAYLOAD_TYPE, MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MF_MT_AUDIO_BITS_PER_SAMPLE,
    MF_MT_AUDIO_BLOCK_ALIGNMENT, MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND,
    MF_MT_AVG_BITRATE, MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE,
    MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE, MF_MT_MPEG2_PROFILE, MF_MT_PIXEL_ASPECT_RATIO,
    MF_MT_SUBTYPE, MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, MF_TRANSCODE_CONTAINERTYPE,
    MFAudioFormat_AAC, MFAudioFormat_PCM, MFCreateAttributes, MFCreateMediaType,
    MFCreateMemoryBuffer, MFCreateSample, MFCreateSinkWriterFromURL, MFMediaType_Audio,
    MFMediaType_Video, MFTranscodeContainerType_MPEG4, MFVideoFormat_H264, MFVideoFormat_NV12,
    MFVideoInterlace_Progressive, eAVEncH264VProfile_High,
};
use windows::core::{GUID, HSTRING};

use super::reader::AudioInfo;
use super::{TranscodeError, failed};
use crate::h264::colour;

const AAC_LC_PROFILE_LEVEL: u32 = 0x29;
const RAW_PAYLOAD: u32 = 0;
const PCM_BITS: u32 = 16;

pub(super) struct Sink {
    writer: IMFSinkWriter,
    video: u32,
    audio: Option<u32>,
}

fn pack(high: u32, low: u32) -> u64 {
    (u64::from(high) << 32) | u64::from(low)
}

fn new_type(major: &GUID, subtype: &GUID) -> Result<IMFMediaType, TranscodeError> {
    // SAFETY: creates a media type and sets two GUID attributes on the live object.
    unsafe {
        let media = MFCreateMediaType().map_err(|e| failed("media type", &e))?;
        media
            .SetGUID(&MF_MT_MAJOR_TYPE, major)
            .map_err(|e| failed("media type", &e))?;
        media
            .SetGUID(&MF_MT_SUBTYPE, subtype)
            .map_err(|e| failed("media type", &e))?;
        Ok(media)
    }
}

fn video_type(
    plan: &EncodePlan,
    subtype: &GUID,
    encoded: bool,
) -> Result<IMFMediaType, TranscodeError> {
    let media = new_type(&MFMediaType_Video, subtype)?;
    // SAFETY: attribute writes on a live media type.
    let set = unsafe {
        media
            .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .and_then(|()| media.SetUINT64(&MF_MT_FRAME_SIZE, pack(plan.width, plan.height)))
            .and_then(|()| media.SetUINT64(&MF_MT_FRAME_RATE, pack(plan.fps, 1)))
            .and_then(|()| media.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1)))
            .and_then(|()| {
                if encoded {
                    media
                        .SetUINT32(&MF_MT_AVG_BITRATE, plan.video_bps)
                        .and_then(|()| {
                            media.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)
                        })
                } else {
                    media.SetUINT32(&MF_MT_DEFAULT_STRIDE, plan.width)
                }
            })
    };
    set.and_then(|()| colour::apply(&media))
        .map_err(|e| failed("video type", &e))?;
    Ok(media)
}

fn audio_type(
    info: AudioInfo,
    subtype: &GUID,
    bytes_per_second: u32,
    encoded: bool,
) -> Result<IMFMediaType, TranscodeError> {
    let media = new_type(&MFMediaType_Audio, subtype)?;
    // SAFETY: attribute writes on a live media type.
    let set = unsafe {
        media
            .SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, PCM_BITS)
            .and_then(|()| media.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, info.sample_rate))
            .and_then(|()| media.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, info.channels))
            .and_then(|()| media.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, bytes_per_second))
            .and_then(|()| {
                if encoded {
                    media
                        .SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, RAW_PAYLOAD)
                        .and_then(|()| {
                            media.SetUINT32(
                                &MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION,
                                AAC_LC_PROFILE_LEVEL,
                            )
                        })
                } else {
                    media.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, info.channels * PCM_BITS / 8)
                }
            })
    };
    set.map_err(|e| failed("audio type", &e))?;
    Ok(media)
}

fn writer_attributes(hardware: bool) -> Result<IMFAttributes, TranscodeError> {
    let mut attributes: Option<IMFAttributes> = None;
    // SAFETY: attribute construction into a local that lives through the calls.
    unsafe {
        MFCreateAttributes(&mut attributes, 3).map_err(|e| failed("attributes", &e))?;
        let attributes =
            attributes.ok_or_else(|| TranscodeError::Failed("cannot create attributes".into()))?;
        attributes
            .SetUINT32(
                &MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS,
                u32::from(hardware),
            )
            .map_err(|e| failed("attributes", &e))?;
        attributes
            .SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4)
            .map_err(|e| failed("attributes", &e))?;
        attributes
            .SetUINT32(&MF_MPEG4SINK_MOOV_BEFORE_MDAT, 1)
            .map_err(|e| failed("attributes", &e))?;
        Ok(attributes)
    }
}

impl Sink {
    pub(super) fn create(
        path: &Path,
        plan: &EncodePlan,
        audio: Option<AudioInfo>,
        hardware: bool,
    ) -> Result<Self, TranscodeError> {
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let attributes = writer_attributes(hardware)?;
        // SAFETY: the URL and attributes outlive the call; no byte stream is passed.
        let writer = unsafe {
            MFCreateSinkWriterFromURL(
                &HSTRING::from(absolute.as_os_str()),
                None::<&IMFByteStream>,
                &attributes,
            )
        }
        .map_err(|e| failed("create the file", &e))?;
        let h264 = video_type(plan, &MFVideoFormat_H264, true)?;
        let nv12 = video_type(plan, &MFVideoFormat_NV12, false)?;
        // SAFETY: stream setup on a live writer with live media types.
        let video = unsafe {
            let index = writer
                .AddStream(&h264)
                .map_err(|e| failed("add the video stream", &e))?;
            writer
                .SetInputMediaType(index, &nv12, None::<&IMFAttributes>)
                .map_err(|e| failed("video encoder input", &e))?;
            index
        };
        let audio = match audio {
            Some(info) => Some(Self::add_audio(&writer, info, plan.audio_bps)?),
            None => None,
        };
        // SAFETY: both streams are configured, so writing may begin.
        unsafe { writer.BeginWriting() }.map_err(|e| failed("start the encoder", &e))?;
        Ok(Self {
            writer,
            video,
            audio,
        })
    }

    fn add_audio(
        writer: &IMFSinkWriter,
        info: AudioInfo,
        audio_bps: u32,
    ) -> Result<u32, TranscodeError> {
        let aac = audio_type(info, &MFAudioFormat_AAC, audio_bps / 8, true)?;
        let pcm = audio_type(
            info,
            &MFAudioFormat_PCM,
            info.sample_rate * info.channels * PCM_BITS / 8,
            false,
        )?;
        // SAFETY: stream setup on a live writer with live media types.
        unsafe {
            let index = writer
                .AddStream(&aac)
                .map_err(|e| failed("add the audio stream", &e))?;
            writer
                .SetInputMediaType(index, &pcm, None::<&IMFAttributes>)
                .map_err(|e| failed("audio encoder input", &e))?;
            Ok(index)
        }
    }

    pub(super) fn write_video(
        &self,
        nv12: &[u8],
        time: i64,
        duration: i64,
    ) -> Result<(), TranscodeError> {
        let length = u32::try_from(nv12.len())
            .map_err(|_| TranscodeError::Failed("frame too large".into()))?;
        // SAFETY: the buffer holds `length` bytes, is locked for the copy, and is unlocked before use.
        unsafe {
            let buffer = MFCreateMemoryBuffer(length).map_err(|e| failed("frame buffer", &e))?;
            let mut data = std::ptr::null_mut();
            buffer
                .Lock(&mut data, None, None)
                .map_err(|e| failed("frame buffer", &e))?;
            std::ptr::copy_nonoverlapping(nv12.as_ptr(), data, nv12.len());
            buffer.Unlock().map_err(|e| failed("frame buffer", &e))?;
            buffer
                .SetCurrentLength(length)
                .map_err(|e| failed("frame buffer", &e))?;
            let sample = MFCreateSample().map_err(|e| failed("frame sample", &e))?;
            sample
                .AddBuffer(&buffer)
                .map_err(|e| failed("frame sample", &e))?;
            sample
                .SetSampleTime(time)
                .map_err(|e| failed("frame sample", &e))?;
            sample
                .SetSampleDuration(duration)
                .map_err(|e| failed("frame sample", &e))?;
            self.writer
                .WriteSample(self.video, &sample)
                .map_err(|e| failed("encode a frame", &e))
        }
    }

    pub(super) fn write_audio(&self, sample: &IMFSample, time: i64) -> Result<(), TranscodeError> {
        let Some(stream) = self.audio else {
            return Ok(());
        };
        // SAFETY: the sample is live and owned by this call's reader read; the writer holds its own reference.
        unsafe {
            sample
                .SetSampleTime(time)
                .map_err(|e| failed("audio sample", &e))?;
            self.writer
                .WriteSample(stream, sample)
                .map_err(|e| failed("encode audio", &e))
        }
    }

    pub(super) fn finish(self) -> Result<(), TranscodeError> {
        // SAFETY: flushes and closes the file; the writer is dropped right after.
        unsafe { self.writer.Finalize() }.map_err(|e| failed("finish the file", &e))
    }
}
