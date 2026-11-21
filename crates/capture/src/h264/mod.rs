mod pump;
mod rate;
mod setup;
#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use greyscreen_core::media::CaptureConfig;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Media::MediaFoundation::{
    CODECAPI_AVEncVideoForceKeyFrame, ICodecAPI, IMFActivate, IMFDXGIDeviceManager, IMFMediaBuffer,
    IMFMediaEventGenerator, IMFSample, IMFShutdown, IMFTransform, MFCreateDXGISurfaceBuffer,
    MFCreateSample, MFT_MESSAGE_COMMAND_DRAIN, MFT_MESSAGE_COMMAND_FLUSH,
    MFT_MESSAGE_NOTIFY_END_OF_STREAM, MFT_MESSAGE_NOTIFY_END_STREAMING,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::Interface;

use crate::d3d::GpuDevice;
use crate::vproc::{Nv12Frame, SlotReleaser};

const DRAIN_TIMEOUT: Duration = Duration::from_secs(3);
const DRAIN_POLL: Duration = Duration::from_millis(2);
const PROBE_SIZE: (u32, u32) = (1920, 1080);
const PROBE_FPS: u32 = 60;

#[derive(Debug, thiserror::Error)]
pub enum H264Error {
    #[error("no usable hardware H.264 encoder: {0}")]
    NoHardwareEncoder(String),
    #[error("invalid encoder configuration: {0}")]
    Config(String),
    #[error("encoder rejected required property {property}: {source}")]
    Property {
        property: &'static str,
        source: windows::core::Error,
    },
    #[error("encoder is not waiting for input")]
    NotAccepting,
    #[error("encoder produced a bitstream that is neither Annex B nor length-prefixed")]
    Bitstream,
    #[error("media foundation call failed: {0}")]
    Windows(#[from] windows::core::Error),
}

pub type Result<T> = std::result::Result<T, H264Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedVideo {
    pub annexb: Vec<u8>,
    pub pts_hns: i64,
    pub key: bool,
}

#[derive(Debug)]
pub struct H264Encoder {
    scratch: Option<IMFSample>,
    events: IMFMediaEventGenerator,
    codec: ICodecAPI,
    transform: IMFTransform,
    activate: IMFActivate,
    _manager: IMFDXGIDeviceManager,
    name: String,
    sequence_header: Vec<u8>,
    input_id: u32,
    output_id: u32,
    provides_samples: bool,
    frame_bytes: u32,
    pending_need_input: u32,
    pending_have_output: u32,
    drain_complete: bool,
    inflight: VecDeque<(usize, i64)>,
    releaser: SlotReleaser,
    shut_down: bool,
    _thread_bound: PhantomData<*const ()>,
}

impl H264Encoder {
    pub fn new(
        gpu: &GpuDevice,
        width: u32,
        height: u32,
        fps: u32,
        cfg: &CaptureConfig,
    ) -> Result<Self> {
        if width == 0
            || height == 0
            || !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
            || fps == 0
        {
            return Err(H264Error::Config(format!(
                "size {width}x{height} at {fps} fps must be non-zero with even sides"
            )));
        }
        let spec = setup::Spec {
            width,
            height,
            fps,
            mean_bitrate: cfg.video_bitrate,
            peak_bitrate: cfg.video_peak.max(cfg.video_bitrate),
            quality: cfg.quality,
            gop_frames: cfg.gop_frames,
        };
        let opened = setup::open(gpu, &spec)?;
        let mut encoder = Self {
            scratch: None,
            events: opened.events,
            codec: opened.codec,
            transform: opened.transform,
            activate: opened.activate,
            _manager: opened.manager,
            name: opened.name,
            sequence_header: opened.sequence_header,
            input_id: opened.input_id,
            output_id: opened.output_id,
            provides_samples: false,
            frame_bytes: width * height * 3 / 2,
            pending_need_input: 0,
            pending_have_output: 0,
            drain_complete: false,
            inflight: VecDeque::new(),
            releaser: SlotReleaser::default(),
            shut_down: false,
            _thread_bound: PhantomData,
        };
        encoder.refresh_output_stream()?;
        Ok(encoder)
    }

    pub fn probe(gpu: &GpuDevice) -> Result<String> {
        let cfg = CaptureConfig {
            fps: PROBE_FPS,
            max_height: None,
            video_bitrate: 20_000_000,
            video_peak: 30_000_000,
            quality: None,
            gop_frames: 120,
            audio: false,
            audio_bitrate: 0,
            cursor: false,
            bitrate_pixels: 0,
            diagnostics: false,
        };
        match Self::new(gpu, PROBE_SIZE.0, PROBE_SIZE.1, PROBE_FPS, &cfg) {
            Ok(encoder) => {
                let name = encoder.encoder_name();
                tracing::info!(encoder = %name, "hardware H.264 encoder probe succeeded");
                Ok(name)
            }
            Err(error) => {
                tracing::warn!(%error, "hardware H.264 encoder probe failed");
                Err(error)
            }
        }
    }

    pub fn with_releaser(mut self, releaser: SlotReleaser) -> Self {
        self.releaser = releaser;
        self
    }

    pub fn encoder_name(&self) -> String {
        self.name.clone()
    }

    pub fn sequence_header(&self) -> &[u8] {
        &self.sequence_header
    }

    pub fn need_input(&mut self) -> Result<bool> {
        self.pump_events()?;
        Ok(self.pending_need_input > 0)
    }

    pub fn submit(&mut self, frame: &Nv12Frame, pts_hns: i64, dur_hns: i64) -> Result<()> {
        self.pump_events()?;
        if self.pending_need_input == 0 {
            return Err(H264Error::NotAccepting);
        }
        let sample = self.build_sample(&frame.texture, pts_hns, dur_hns)?;
        // SAFETY: the transform is live and the sample outlives the call.
        unsafe { self.transform.ProcessInput(self.input_id, &sample, 0)? };
        self.pending_need_input -= 1;
        self.inflight.push_back((frame.slot, pts_hns));
        Ok(())
    }

    pub fn force_keyframe(&mut self) -> Result<()> {
        let value = VARIANT::from(1u32);
        // SAFETY: the codec interface is live and `value` outlives the call.
        unsafe {
            self.codec
                .SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &value)?;
        }
        Ok(())
    }

    pub fn drain_output(&mut self, out: &mut Vec<EncodedVideo>) -> Result<()> {
        self.pump_events()?;
        while self.pending_have_output > 0 {
            self.pending_have_output -= 1;
            if let Some(packet) = self.process_output()? {
                out.push(packet);
            }
        }
        Ok(())
    }

    pub fn finish(mut self, out: &mut Vec<EncodedVideo>) -> Result<()> {
        let result = self.drain_to_end(out);
        self.shutdown();
        result
    }

    fn drain_to_end(&mut self, out: &mut Vec<EncodedVideo>) -> Result<()> {
        // SAFETY: the transform is live and the message carries no parameter.
        unsafe {
            self.transform
                .ProcessMessage(MFT_MESSAGE_COMMAND_DRAIN, 0)?;
        }
        let deadline = Instant::now() + DRAIN_TIMEOUT;
        loop {
            self.drain_output(out)?;
            if self.drain_complete && self.pending_have_output == 0 {
                return Ok(());
            }
            if Instant::now() >= deadline {
                tracing::warn!("encoder drain timed out");
                return Ok(());
            }
            std::thread::sleep(DRAIN_POLL);
        }
    }

    fn build_sample(
        &self,
        texture: &ID3D11Texture2D,
        pts_hns: i64,
        dur_hns: i64,
    ) -> Result<IMFSample> {
        // SAFETY: the texture is a live NV12 texture on the encoder's device.
        let buffer: IMFMediaBuffer =
            unsafe { MFCreateDXGISurfaceBuffer(&ID3D11Texture2D::IID, texture, 0, false)? };
        // SAFETY: the buffer was just created and is not shared.
        unsafe { buffer.SetCurrentLength(self.frame_bytes)? };
        // SAFETY: MFCreateSample has no pointer arguments.
        let sample = unsafe { MFCreateSample()? };
        // SAFETY: the sample and buffer are live and owned by this call.
        unsafe {
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(pts_hns)?;
            sample.SetSampleDuration(dur_hns)?;
        }
        Ok(sample)
    }

    fn release_inflight(&mut self) {
        while let Some((slot, _)) = self.inflight.pop_front() {
            self.releaser.release(slot);
        }
    }

    fn shutdown(&mut self) {
        if self.shut_down {
            return;
        }
        self.shut_down = true;
        for message in [
            MFT_MESSAGE_NOTIFY_END_OF_STREAM,
            MFT_MESSAGE_NOTIFY_END_STREAMING,
            MFT_MESSAGE_COMMAND_FLUSH,
        ] {
            // SAFETY: the transform is live and the messages carry no parameter.
            if let Err(error) = unsafe { self.transform.ProcessMessage(message, 0) } {
                tracing::debug!(%error, "encoder shutdown message failed");
            }
        }
        self.release_inflight();
        if let Ok(shutdown) = self.transform.cast::<IMFShutdown>() {
            // SAFETY: the interface is live; Shutdown ends the event queue.
            if let Err(error) = unsafe { shutdown.Shutdown() } {
                tracing::debug!(%error, "encoder IMFShutdown failed");
            }
        }
        // SAFETY: the activate object created this transform and is still live.
        if let Err(error) = unsafe { self.activate.ShutdownObject() } {
            tracing::debug!(%error, "encoder activate shutdown failed");
        }
    }
}

impl Drop for H264Encoder {
    fn drop(&mut self) {
        self.shutdown();
    }
}
