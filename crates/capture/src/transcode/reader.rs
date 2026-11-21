use std::path::Path;

use greyscreen_core::nv12::{Nv12Source, scale};
use windows::Win32::Media::MediaFoundation::{
    IMF2DBuffer2, IMFAttributes, IMFMediaType, IMFSample, IMFSourceReader,
    MF_E_INVALIDSTREAMNUMBER, MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_NUM_CHANNELS,
    MF_MT_AUDIO_SAMPLES_PER_SECOND, MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE,
    MF_MT_MINIMUM_DISPLAY_APERTURE, MF_MT_SUBTYPE, MF_SOURCE_READER_ALL_STREAMS,
    MF_SOURCE_READER_ANY_STREAM, MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING,
    MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED, MF_SOURCE_READERF_ENDOFSTREAM,
    MF2DBuffer_LockFlags_Read, MFAudioFormat_PCM, MFCreateAttributes, MFCreateMediaType,
    MFCreateSourceReaderFromURL, MFMediaType_Audio, MFMediaType_Video, MFVideoArea,
    MFVideoFormat_NV12,
};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Variant::VT_I8;
use windows::core::{GUID, HSTRING, Interface};

use super::{TranscodeError, failed};

const PCM_BITS: u32 = 16;

#[derive(Debug, Clone, Copy)]
pub(super) struct AudioInfo {
    pub(super) sample_rate: u32,
    pub(super) channels: u32,
}

#[derive(Debug, Clone, Copy)]
struct Picture {
    coded_width: usize,
    crop: (usize, usize, usize, usize),
}

pub(super) enum Item {
    Video { sample: IMFSample, time: i64 },
    Audio { sample: IMFSample, time: i64 },
    End,
}

pub(super) struct Source {
    reader: IMFSourceReader,
    video: u32,
    audio: Option<(u32, AudioInfo)>,
    picture: Picture,
    video_ended: bool,
    audio_ended: bool,
}

fn media_type(major: &GUID, subtype: &GUID) -> Result<IMFMediaType, TranscodeError> {
    // SAFETY: creates a media type and sets two GUID attributes on the live object.
    unsafe {
        let wanted = MFCreateMediaType().map_err(|e| failed("media type", &e))?;
        wanted
            .SetGUID(&MF_MT_MAJOR_TYPE, major)
            .map_err(|e| failed("media type", &e))?;
        wanted
            .SetGUID(&MF_MT_SUBTYPE, subtype)
            .map_err(|e| failed("media type", &e))?;
        Ok(wanted)
    }
}

fn find_streams(reader: &IMFSourceReader) -> (Option<u32>, Option<u32>) {
    let (mut video, mut audio) = (None, None);
    for index in 0..16u32 {
        // SAFETY: a read-only query on a live reader; an unknown index returns an error.
        let native = match unsafe { reader.GetNativeMediaType(index, 0) } {
            Ok(native) => native,
            Err(err) if err.code() == MF_E_INVALIDSTREAMNUMBER => break,
            Err(_) => continue,
        };
        // SAFETY: attribute read on a live media type.
        let major = unsafe { native.GetGUID(&MF_MT_MAJOR_TYPE) };
        match major {
            Ok(kind) if kind == MFMediaType_Video && video.is_none() => video = Some(index),
            Ok(kind) if kind == MFMediaType_Audio && audio.is_none() => audio = Some(index),
            _ => {}
        }
    }
    (video, audio)
}

impl Source {
    pub(super) fn open(path: &Path, start_hns: i64) -> Result<Self, TranscodeError> {
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let mut attributes: Option<IMFAttributes> = None;
        // SAFETY: attribute and reader construction with out-pointers to locals that live through the calls.
        let reader = unsafe {
            MFCreateAttributes(&mut attributes, 1).map_err(|e| failed("attributes", &e))?;
            let attributes = attributes
                .ok_or_else(|| TranscodeError::Failed("cannot create attributes".into()))?;
            attributes
                .SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1)
                .map_err(|e| failed("attributes", &e))?;
            MFCreateSourceReaderFromURL(&HSTRING::from(absolute.as_os_str()), &attributes)
                .map_err(|e| failed("open the video", &e))?
        };
        let (video, audio) = find_streams(&reader);
        let video = video.ok_or_else(|| TranscodeError::Failed("the file has no video".into()))?;
        // SAFETY: stream selection and media type requests on a live reader.
        let picture = unsafe {
            reader
                .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)
                .map_err(|e| failed("select streams", &e))?;
            reader
                .SetStreamSelection(video, true)
                .map_err(|e| failed("select video", &e))?;
            reader
                .SetCurrentMediaType(
                    video,
                    None,
                    &media_type(&MFMediaType_Video, &MFVideoFormat_NV12)?,
                )
                .map_err(|e| failed("decode video", &e))?;
            picture(
                &reader
                    .GetCurrentMediaType(video)
                    .map_err(|e| failed("video type", &e))?,
            )?
        };
        let audio = audio.and_then(|index| open_audio(&reader, index));
        seek(&reader, start_hns)?;
        Ok(Self {
            reader,
            video,
            audio,
            picture,
            video_ended: false,
            audio_ended: audio.is_none(),
        })
    }

    pub(super) fn audio(&self) -> Option<AudioInfo> {
        self.audio.map(|(_, info)| info)
    }

    pub(super) fn has_audio(&self) -> bool {
        self.audio.is_some()
    }

    pub(super) fn next(&mut self) -> Result<Item, TranscodeError> {
        loop {
            if self.video_ended && self.audio_ended {
                return Ok(Item::End);
            }
            let (mut index, mut flags, mut time, mut sample) = (0u32, 0u32, 0i64, None);
            // SAFETY: a blocking read into locals that live through the call.
            unsafe {
                self.reader
                    .ReadSample(
                        MF_SOURCE_READER_ANY_STREAM.0 as u32,
                        0,
                        Some(&mut index),
                        Some(&mut flags),
                        Some(&mut time),
                        Some(&mut sample),
                    )
                    .map_err(|e| failed("read the video", &e))?;
            }
            let is_video = index == self.video;
            if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 && is_video {
                // SAFETY: a read-only query on the live reader.
                let current = unsafe { self.reader.GetCurrentMediaType(self.video) }
                    .map_err(|e| failed("video type", &e))?;
                self.picture = picture(&current)?;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                if is_video {
                    self.video_ended = true;
                } else {
                    self.audio_ended = true;
                }
            }
            let Some(sample) = sample else { continue };
            if is_video {
                return Ok(Item::Video { sample, time });
            }
            if self.audio.is_some_and(|(audio, _)| audio == index) {
                return Ok(Item::Audio { sample, time });
            }
        }
    }

    pub(super) fn scale_into(
        &self,
        sample: &IMFSample,
        width: usize,
        height: usize,
        out: &mut Vec<u8>,
    ) -> Result<(), TranscodeError> {
        let bad = || TranscodeError::Failed("unexpected video frame layout".into());
        // SAFETY: the buffer is locked for the copy and unlocked after it.
        unsafe {
            let buffer = sample
                .ConvertToContiguousBuffer()
                .map_err(|e| failed("frame buffer", &e))?;
            let (mut base, mut pitch, mut len) = (std::ptr::null_mut(), 0i32, 0u32);
            let two_d = buffer.cast::<IMF2DBuffer2>().ok();
            let locked = match &two_d {
                Some(plane) => {
                    let mut start = std::ptr::null_mut();
                    plane
                        .Lock2DSize(
                            MF2DBuffer_LockFlags_Read,
                            &mut base,
                            &mut pitch,
                            &mut start,
                            &mut len,
                        )
                        .is_ok()
                }
                None => false,
            };
            if !locked {
                buffer
                    .Lock(&mut base, None, Some(&mut len))
                    .map_err(|e| failed("frame buffer", &e))?;
                pitch = self.picture.coded_width as i32;
            }
            let outcome = self.scale_locked(
                base,
                pitch.unsigned_abs() as usize,
                len as usize,
                width,
                height,
                out,
            );
            match &two_d {
                Some(plane) if locked => {
                    plane.Unlock2D().ok();
                }
                _ => {
                    buffer.Unlock().ok();
                }
            }
            outcome.then_some(()).ok_or_else(bad)
        }
    }

    unsafe fn scale_locked(
        &self,
        base: *mut u8,
        pitch: usize,
        len: usize,
        width: usize,
        height: usize,
        out: &mut Vec<u8>,
    ) -> bool {
        let (_, y, _, h) = self.picture.crop;
        let rows = match len * 2 / (3 * pitch.max(1)) {
            0 => y + h,
            rows => rows.max(y + h),
        };
        if base.is_null() || pitch == 0 || len < rows * pitch * 3 / 2 {
            return false;
        }
        // SAFETY: the locked buffer holds `len` readable bytes, and `rows * pitch * 3 / 2 <= len` was checked.
        let data = unsafe { std::slice::from_raw_parts(base, rows * pitch * 3 / 2) };
        scale(
            &Nv12Source {
                data,
                pitch,
                rows,
                crop: self.picture.crop,
            },
            width,
            height,
            out,
        )
    }
}

fn open_audio(reader: &IMFSourceReader, index: u32) -> Option<(u32, AudioInfo)> {
    let wanted = media_type(&MFMediaType_Audio, &MFAudioFormat_PCM).ok()?;
    // SAFETY: attribute writes on a live media type and requests on a live reader.
    let info = unsafe {
        wanted
            .SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, PCM_BITS)
            .ok()?;
        reader.SetStreamSelection(index, true).ok()?;
        reader.SetCurrentMediaType(index, None, &wanted).ok()?;
        let current = reader.GetCurrentMediaType(index).ok()?;
        AudioInfo {
            sample_rate: current.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND).ok()?,
            channels: current.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).ok()?,
        }
    };
    let supported = matches!(info.channels, 1 | 2) && matches!(info.sample_rate, 44_100 | 48_000);
    if !supported {
        tracing::warn!(
            ?info,
            "the audio format cannot be re-encoded, exporting without sound"
        );
        // SAFETY: stream selection on a live reader.
        unsafe { reader.SetStreamSelection(index, false) }.ok();
        return None;
    }
    Some((index, info))
}

fn picture(media: &IMFMediaType) -> Result<Picture, TranscodeError> {
    // SAFETY: attribute reads on a live media type; the blob buffer is exactly one MFVideoArea.
    unsafe {
        let size = media
            .GetUINT64(&MF_MT_FRAME_SIZE)
            .map_err(|e| failed("video size", &e))?;
        let (width, height) = ((size >> 32) as usize, size as u32 as usize);
        if width < 2 || height < 2 {
            return Err(TranscodeError::Failed("the video has no size".into()));
        }
        let mut area = MFVideoArea::default();
        let bytes =
            std::slice::from_raw_parts_mut((&raw mut area).cast::<u8>(), size_of::<MFVideoArea>());
        let crop = match media.GetBlob(&MF_MT_MINIMUM_DISPLAY_APERTURE, bytes, None) {
            Ok(()) if area.Area.cx > 0 && area.Area.cy > 0 => {
                let x = area.OffsetX.value.max(0) as usize & !1;
                let y = area.OffsetY.value.max(0) as usize & !1;
                let w = (area.Area.cx as usize).min(width.saturating_sub(x)) & !1;
                let h = (area.Area.cy as usize).min(height.saturating_sub(y)) & !1;
                (x, y, w.max(2), h.max(2))
            }
            _ => (0, 0, width & !1, height & !1),
        };
        Ok(Picture {
            coded_width: width,
            crop,
        })
    }
}

fn seek(reader: &IMFSourceReader, start_hns: i64) -> Result<(), TranscodeError> {
    let mut at = PROPVARIANT::default();
    // SAFETY: a VT_I8 PROPVARIANT owns nothing, so it needs no clearing.
    unsafe {
        (*at.Anonymous.Anonymous).vt = VT_I8;
        (*at.Anonymous.Anonymous).Anonymous.hVal = start_hns;
        reader
            .SetCurrentPosition(&GUID::zeroed(), &at)
            .map_err(|e| failed("seek", &e))
    }
}
