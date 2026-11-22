use wasapi::{
    AudioCaptureClient, AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode,
    WasapiError, WaveFormat,
};
use windows::Win32::Media::Audio::AUDCLNT_E_DEVICE_INVALIDATED;

use super::{AudioError, AudioTarget, CHANNELS, RATE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacketInfo {
    pub discontinuity: bool,
    pub silent: bool,
    pub timestamp_error: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SourceError {
    #[error("audio device invalidated")]
    DeviceInvalidated,
    #[error("{0}")]
    Failed(String),
}

impl From<WasapiError> for SourceError {
    fn from(error: WasapiError) -> Self {
        match error {
            WasapiError::Windows(inner) if inner.code() == AUDCLNT_E_DEVICE_INVALIDATED => {
                Self::DeviceInvalidated
            }
            other => Self::Failed(other.to_string()),
        }
    }
}

pub trait CaptureClient {
    fn wait(&mut self, timeout_ms: u32) -> Result<bool, SourceError>;
    fn next_packet_size(&mut self) -> Result<Option<u32>, SourceError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<(u32, PacketInfo), SourceError>;
    fn stop(&mut self);
}

pub type OpenSource = Box<dyn FnOnce() -> Result<Box<dyn CaptureClient>, AudioError> + Send>;

struct Device {
    client: AudioClient,
    capture: AudioCaptureClient,
    event: wasapi::Handle,
}

pub fn open_device(target: AudioTarget) -> OpenSource {
    Box::new(move || Ok(Box::new(Device::open(target)?)))
}

impl Device {
    fn open(target: AudioTarget) -> Result<Self, AudioError> {
        let mut client = match target {
            AudioTarget::Process(pid) => AudioClient::new_application_loopback_client(pid, true)?,
            AudioTarget::System => DeviceEnumerator::new()?
                .get_default_device(Direction::Render)?
                .get_iaudioclient()?,
        };
        let format = WaveFormat::new(32, 32, SampleType::Float, RATE as usize, CHANNELS, None);
        client.initialize_client(
            &format,
            Direction::Capture,
            StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 0,
            },
            None,
        )?;
        let event = client.set_get_eventhandle()?;
        let capture = client.get_audiocaptureclient()?;
        client.start_stream()?;
        Ok(Self {
            client,
            capture,
            event,
        })
    }
}

impl CaptureClient for Device {
    fn wait(&mut self, timeout_ms: u32) -> Result<bool, SourceError> {
        match self.event.wait_for_event(timeout_ms) {
            Ok(()) => Ok(true),
            Err(WasapiError::EventTimeout) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn next_packet_size(&mut self) -> Result<Option<u32>, SourceError> {
        Ok(self.capture.get_next_packet_size()?)
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<(u32, PacketInfo), SourceError> {
        let (frames, info) = self.capture.read_from_device(buffer)?;
        Ok((
            frames,
            PacketInfo {
                discontinuity: info.flags.data_discontinuity,
                silent: info.flags.silent,
                timestamp_error: info.flags.timestamp_error,
                timestamp: info.timestamp,
            },
        ))
    }

    fn stop(&mut self) {
        if let Err(error) = self.client.stop_stream() {
            tracing::warn!(%error, "stopping audio stream failed");
        }
        if let Err(error) = self.client.reset_stream() {
            tracing::warn!(%error, "resetting audio stream failed");
        }
    }
}
