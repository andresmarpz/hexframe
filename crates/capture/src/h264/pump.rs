use std::mem::ManuallyDrop;

use greyscreen_mp4::avc::{NAL_IDR, NAL_PREFIX, nal_type, split_annexb};
use windows::Win32::Media::MediaFoundation::{
    IMFMediaBuffer, IMFSample, MEError, METransformDrainComplete, METransformHaveOutput,
    METransformNeedInput, MF_E_NO_EVENTS_AVAILABLE, MF_E_TRANSFORM_NEED_MORE_INPUT,
    MF_E_TRANSFORM_STREAM_CHANGE, MF_MT_MPEG_SEQUENCE_HEADER, MFCreateMemoryBuffer, MFCreateSample,
    MFT_OUTPUT_DATA_BUFFER, MFT_OUTPUT_STREAM_PROVIDES_SAMPLES,
};

use super::{EncodedVideo, H264Encoder, H264Error, Mode, Result};

const START_CODE: [u8; 4] = [0, 0, 0, 1];
const MIN_OUTPUT_BUFFER: u32 = 1 << 20;
const MAX_OUTPUT_ATTEMPTS: u32 = 2;
const MAX_OUTPUT_TYPES: u32 = 64;

impl H264Encoder {
    pub(super) fn pump_events(&mut self) -> Result<()> {
        if !matches!(self.mode, Mode::Async { .. }) {
            return Ok(());
        }
        loop {
            let event = match self.mft.next_event() {
                Ok(event) => event,
                Err(error) if error.code() == MF_E_NO_EVENTS_AVAILABLE => return Ok(()),
                Err(error) => return Err(error.into()),
            };
            // SAFETY: the event was just returned and is live.
            let kind = unsafe { event.GetType()? } as i32;
            match kind {
                k if k == METransformNeedInput.0 => self.pending_need_input += 1,
                k if k == METransformHaveOutput.0 => self.pending_have_output += 1,
                k if k == METransformDrainComplete.0 => self.drain_complete = true,
                k if k == MEError.0 => {
                    // SAFETY: the event is live.
                    let status = unsafe { event.GetStatus()? };
                    return Err(H264Error::Stopped(windows::core::Error::from(status)));
                }
                _ => {}
            }
        }
    }

    pub(super) fn refresh_output_stream(&mut self) -> Result<()> {
        let info = self.mft.output_stream_info(self.output_id)?;
        self.provides_samples = info.dwFlags & (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32) != 0;
        self.scratch = if self.provides_samples {
            None
        } else {
            Some(create_output_sample(info.cbSize.max(MIN_OUTPUT_BUFFER))?)
        };
        Ok(())
    }

    pub(super) fn process_output(&mut self) -> Result<Option<EncodedVideo>> {
        for _ in 0..MAX_OUTPUT_ATTEMPTS {
            let mut buffer = MFT_OUTPUT_DATA_BUFFER {
                dwStreamID: self.output_id,
                pSample: ManuallyDrop::new(self.scratch.clone()),
                ..Default::default()
            };
            let mut status = 0u32;
            let result = self
                .mft
                .process_output(std::slice::from_mut(&mut buffer), &mut status);
            // SAFETY: the buffer owns the sample reference placed or returned by the call and is not used again.
            let sample = unsafe { ManuallyDrop::take(&mut buffer.pSample) };
            // SAFETY: the buffer owns any returned event collection and is not used again.
            unsafe { ManuallyDrop::drop(&mut buffer.pEvents) };
            match result {
                Ok(()) => {
                    return match sample {
                        Some(sample) => self.finish_output(&sample).map(Some),
                        None => Ok(None),
                    };
                }
                Err(error) if error.code() == MF_E_TRANSFORM_STREAM_CHANGE => {
                    self.renegotiate_output()?;
                }
                Err(error) if error.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => return Ok(None),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(None)
    }

    fn renegotiate_output(&mut self) -> Result<()> {
        let mut last_error = None;
        for index in 0..MAX_OUTPUT_TYPES {
            let available = match self.mft.output_available_type(self.output_id, index) {
                Ok(available) => available,
                Err(error) => {
                    last_error.get_or_insert(error);
                    break;
                }
            };
            match self.mft.set_output_type(self.output_id, &available) {
                Ok(()) => {
                    tracing::info!(index, "encoder output type renegotiated");
                    self.reload_sequence_header();
                    return self.refresh_output_stream();
                }
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error
            .unwrap_or_else(|| windows::core::Error::from(MF_E_TRANSFORM_STREAM_CHANGE))
            .into())
    }

    fn reload_sequence_header(&mut self) {
        let Ok(current) = self.mft.output_current_type(self.output_id) else {
            return;
        };
        // SAFETY: the type is live.
        let Ok(size) = (unsafe { current.GetBlobSize(&MF_MT_MPEG_SEQUENCE_HEADER) }) else {
            return;
        };
        let mut header = vec![0u8; size as usize];
        // SAFETY: `header` has exactly `size` bytes.
        if unsafe { current.GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut header, None) }.is_ok() {
            self.sequence_header = header;
        }
    }

    fn finish_output(&mut self, sample: &IMFSample) -> Result<EncodedVideo> {
        let raw = read_sample_bytes(sample)?;
        // SAFETY: the sample is live.
        let pts_hns = match unsafe { sample.GetSampleTime() } {
            Ok(time) => time,
            Err(error) => {
                tracing::warn!(%error, "encoder output sample has no time");
                self.inflight.front().map_or(0, |entry| entry.1)
            }
        };
        self.reset_scratch();
        self.release_oldest();
        let annexb = to_annexb(&raw)?;
        let key = contains_idr(&annexb);
        Ok(EncodedVideo {
            annexb,
            pts_hns,
            key,
        })
    }

    fn reset_scratch(&self) {
        let Some(scratch) = &self.scratch else {
            return;
        };
        // SAFETY: the scratch sample is live and idle after its bytes were copied out.
        let reset = unsafe {
            scratch
                .GetBufferByIndex(0)
                .and_then(|buffer| buffer.SetCurrentLength(0))
        };
        if let Err(error) = reset {
            tracing::debug!(%error, "scratch buffer reset failed");
        }
    }

    fn release_oldest(&mut self) {
        if let Some((slot, _)) = self.inflight.pop_front() {
            self.releaser.release(slot);
        }
    }
}

fn create_output_sample(bytes: u32) -> Result<IMFSample> {
    // SAFETY: MFCreateMemoryBuffer has no pointer arguments.
    let buffer: IMFMediaBuffer = unsafe { MFCreateMemoryBuffer(bytes)? };
    // SAFETY: MFCreateSample has no pointer arguments.
    let sample = unsafe { MFCreateSample()? };
    // SAFETY: both objects are live and owned by this call.
    unsafe { sample.AddBuffer(&buffer)? };
    Ok(sample)
}

fn read_sample_bytes(sample: &IMFSample) -> Result<Vec<u8>> {
    // SAFETY: the sample is live.
    let buffer = unsafe { sample.ConvertToContiguousBuffer()? };
    let mut data = std::ptr::null_mut();
    let mut length = 0u32;
    // SAFETY: the out pointers refer to live locals.
    unsafe { buffer.Lock(&mut data, None, Some(&mut length))? };
    let bytes = if data.is_null() {
        Vec::new()
    } else {
        // SAFETY: Lock returned `length` readable bytes at `data` until Unlock.
        unsafe { std::slice::from_raw_parts(data, length as usize) }.to_vec()
    };
    // SAFETY: balances the successful Lock above.
    if let Err(error) = unsafe { buffer.Unlock() } {
        tracing::debug!(%error, "output buffer unlock failed");
    }
    Ok(bytes)
}

fn contains_idr(annexb: &[u8]) -> bool {
    split_annexb(annexb).any(|nal| nal.first().is_some_and(|&h| nal_type(h) == NAL_IDR))
}

fn starts_with_start_code(data: &[u8]) -> bool {
    data.starts_with(&[0, 0, 1]) || data.starts_with(&START_CODE)
}

fn push_nal(out: &mut Vec<u8>, nal: &[u8]) {
    if nal.first().is_some_and(|&h| nal_type(h) == NAL_PREFIX) {
        return;
    }
    out.extend_from_slice(&START_CODE);
    out.extend_from_slice(nal);
}

fn length_prefixed_nals(data: &[u8]) -> Option<Vec<&[u8]>> {
    let mut nals = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let (prefix, tail) = rest.split_first_chunk::<4>()?;
        let length = u32::from_be_bytes(*prefix) as usize;
        if length == 0 || length > tail.len() {
            return None;
        }
        let (nal, next) = tail.split_at(length);
        nals.push(nal);
        rest = next;
    }
    Some(nals)
}

pub(super) fn to_annexb(raw: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(raw.len() + 8);
    if starts_with_start_code(raw) {
        for nal in split_annexb(raw) {
            push_nal(&mut out, nal);
        }
    } else {
        let nals = length_prefixed_nals(raw).ok_or(H264Error::Bitstream)?;
        for nal in nals {
            push_nal(&mut out, nal);
        }
    }
    if out.is_empty() {
        return Err(H264Error::Bitstream);
    }
    Ok(out)
}
