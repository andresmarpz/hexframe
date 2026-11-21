use std::path::Path;

use windows::Win32::Media::MediaFoundation::{
    IMF2DBuffer2, IMFAttributes, IMFMediaType, IMFSample, IMFSourceReader, MF_MT_FRAME_SIZE,
    MF_MT_MAJOR_TYPE, MF_MT_MINIMUM_DISPLAY_APERTURE, MF_MT_SUBTYPE, MF_MT_VIDEO_NOMINAL_RANGE,
    MF_SOURCE_READER_ALL_STREAMS, MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING,
    MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_SOURCE_READERF_ENDOFSTREAM, MF_VERSION,
    MF2DBuffer_LockFlags_Read, MFCreateAttributes, MFCreateMediaType, MFCreateSourceReaderFromURL,
    MFMediaType_Video, MFNominalRange_0_255, MFSTARTUP_LITE, MFShutdown, MFStartup, MFVideoArea,
    MFVideoFormat_NV12,
};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Variant::VT_I8;
use windows::core::{GUID, HSTRING, Interface};

use crate::nv12::nv12_to_rgba;
use crate::{Frame, ThumbError};

const HNS: f64 = 10_000_000.0;
const MAX_READS: u32 = 1_200;

struct Startup;

impl Startup {
    fn begin() -> Result<Self, ThumbError> {
        // SAFETY: COM stays initialised on this thread for later calls; a mode clash is harmless.
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .ok();
        // SAFETY: no pointers are passed; the matching MFShutdown runs on drop.
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_LITE) }.map_err(decode)?;
        Ok(Self)
    }
}

impl Drop for Startup {
    fn drop(&mut self) {
        // SAFETY: balances the successful MFStartup in `begin`.
        drop(unsafe { MFShutdown() });
    }
}

fn decode(err: windows::core::Error) -> ThumbError {
    ThumbError::Decode(err.message())
}

#[derive(Clone, Copy)]
struct Picture {
    coded_width: u32,
    crop: (u32, u32, u32, u32),
    full_range: bool,
}

pub(super) fn grab(video: &Path, at_secs: f64, _max_width: u32) -> Result<Frame, ThumbError> {
    if !video.is_file() {
        return Err(ThumbError::Decode(format!(
            "no such file: {}",
            video.display()
        )));
    }
    let _startup = Startup::begin()?;
    let path = std::path::absolute(video).unwrap_or_else(|_| video.to_path_buf());
    let (reader, picture) = open(&path)?;
    seek(&reader, at_secs)?;
    let sample = read_at(&reader, at_secs)?;
    convert(&sample, picture)
}

fn open(path: &Path) -> Result<(IMFSourceReader, Picture), ThumbError> {
    let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
    let mut attributes: Option<IMFAttributes> = None;
    // SAFETY: attribute and reader construction with out-pointers to locals that live through the calls.
    unsafe {
        MFCreateAttributes(&mut attributes, 1).map_err(decode)?;
        let attributes =
            attributes.ok_or_else(|| ThumbError::Decode("cannot create attributes".into()))?;
        attributes
            .SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1)
            .map_err(decode)?;
        let reader = MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attributes)
            .map_err(decode)?;
        drop(reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false));
        reader.SetStreamSelection(stream, true).map_err(decode)?;
        let wanted = MFCreateMediaType().map_err(decode)?;
        wanted
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(decode)?;
        wanted
            .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)
            .map_err(decode)?;
        reader
            .SetCurrentMediaType(stream, None, &wanted)
            .map_err(decode)?;
        let picture = picture(&reader.GetCurrentMediaType(stream).map_err(decode)?)?;
        Ok((reader, picture))
    }
}

fn picture(media: &IMFMediaType) -> Result<Picture, ThumbError> {
    // SAFETY: attribute reads on a live media type; the blob buffer is exactly one MFVideoArea.
    unsafe {
        let size = media
            .GetUINT64(&MF_MT_FRAME_SIZE)
            .map_err(|_| ThumbError::Decode("video track has no size".into()))?;
        let (width, height) = ((size >> 32) as u32, size as u32);
        if width < 2 || height < 2 {
            return Err(ThumbError::Decode("video track has no size".into()));
        }
        let mut area = MFVideoArea::default();
        let bytes =
            std::slice::from_raw_parts_mut((&raw mut area).cast::<u8>(), size_of::<MFVideoArea>());
        let crop = match media.GetBlob(&MF_MT_MINIMUM_DISPLAY_APERTURE, bytes, None) {
            Ok(()) if area.Area.cx > 0 && area.Area.cy > 0 => {
                let x = area.OffsetX.value.max(0) as u32 & !1;
                let y = area.OffsetY.value.max(0) as u32 & !1;
                let w = (area.Area.cx as u32).min(width.saturating_sub(x)) & !1;
                let h = (area.Area.cy as u32).min(height.saturating_sub(y)) & !1;
                (x, y, w.max(2), h.max(2))
            }
            _ => (0, 0, width & !1, height & !1),
        };
        let full_range = media
            .GetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE)
            .is_ok_and(|range| range == MFNominalRange_0_255.0 as u32);
        Ok(Picture {
            coded_width: width,
            crop,
            full_range,
        })
    }
}

fn seek(reader: &IMFSourceReader, secs: f64) -> Result<(), ThumbError> {
    let mut at = PROPVARIANT::default();
    // SAFETY: a VT_I8 PROPVARIANT owns nothing, so it needs no clearing.
    unsafe {
        (*at.Anonymous.Anonymous).vt = VT_I8;
        (*at.Anonymous.Anonymous).Anonymous.hVal = (secs * HNS) as i64;
        reader
            .SetCurrentPosition(&GUID::zeroed(), &at)
            .map_err(decode)
    }
}

fn read_at(reader: &IMFSourceReader, at_secs: f64) -> Result<IMFSample, ThumbError> {
    let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
    let mut latest = None;
    for _ in 0..MAX_READS {
        let (mut flags, mut time, mut sample) = (0u32, 0i64, None);
        // SAFETY: a blocking read into locals.
        unsafe {
            reader
                .ReadSample(
                    stream,
                    0,
                    None,
                    Some(&mut flags),
                    Some(&mut time),
                    Some(&mut sample),
                )
                .map_err(decode)?;
        }
        if let Some(sample) = sample {
            let reached = time as f64 / HNS >= at_secs - 0.02;
            latest = Some(sample);
            if reached {
                break;
            }
        }
        if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
            break;
        }
    }
    latest.ok_or_else(|| ThumbError::Decode("no frame at that time".into()))
}

fn convert(sample: &IMFSample, picture: Picture) -> Result<Frame, ThumbError> {
    // SAFETY: the buffer is locked for the copy and unlocked after it.
    unsafe {
        let buffer = sample.ConvertToContiguousBuffer().map_err(decode)?;
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
                .map_err(decode)?;
            pitch = picture.coded_width as i32;
        }
        let pitch = pitch.unsigned_abs() as usize;
        let (x, y, w, h) = picture.crop;
        let (x, y, w, h) = (x as usize, y as usize, w as usize, h as usize);
        let rows = match len as usize * 2 / (3 * pitch.max(1)) {
            0 => y + h,
            rows => rows.max(y + h),
        };
        let frame = if base.is_null() || pitch == 0 {
            Err(ThumbError::Decode("empty frame buffer".into()))
        } else {
            let data = std::slice::from_raw_parts(base, rows * pitch * 3 / 2);
            Ok(nv12_to_rgba(
                data,
                pitch,
                rows,
                (x, y, w, h),
                picture.full_range,
            ))
        };
        match &two_d {
            Some(plane) if locked => {
                drop(plane.Unlock2D());
            }
            _ => {
                drop(buffer.Unlock());
            }
        }
        frame
    }
}
