use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use wasapi::WasapiError;

use crate::audio::{AudioError, CHANNELS, CaptureClient, OpenSource, PacketInfo, SourceError};

const IDLE_SLEEP: Duration = Duration::from_millis(5);

pub(crate) enum Step {
    Packet { pcm: Vec<f32>, info: PacketInfo },
    Idle,
    WaitFails(SourceError),
    ReadFails(SourceError),
}

impl Step {
    pub(crate) fn packet(pcm: Vec<f32>) -> Self {
        Self::Packet {
            pcm,
            info: PacketInfo::default(),
        }
    }

    pub(crate) fn flagged(pcm: Vec<f32>, info: PacketInfo) -> Self {
        Self::Packet { pcm, info }
    }
}

pub(crate) struct FakeClient {
    steps: VecDeque<Step>,
    stopped: Arc<AtomicBool>,
}

impl FakeClient {
    pub(crate) fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            stopped: Arc::default(),
        }
    }

    pub(crate) fn stopped(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stopped)
    }

    pub(crate) fn open(self) -> OpenSource {
        Box::new(move || Ok(Box::new(self)))
    }
}

pub(crate) fn failing_open() -> OpenSource {
    Box::new(|| Err(AudioError::Wasapi(WasapiError::ClientNotInit)))
}

impl CaptureClient for FakeClient {
    fn wait(&mut self, _timeout_ms: u32) -> Result<bool, SourceError> {
        match self.steps.front() {
            Some(Step::Packet { .. } | Step::ReadFails(_)) => Ok(true),
            Some(Step::Idle) => {
                self.steps.pop_front();
                Ok(false)
            }
            Some(Step::WaitFails(_)) => match self.steps.pop_front() {
                Some(Step::WaitFails(error)) => Err(error),
                _ => Ok(false),
            },
            None => {
                std::thread::sleep(IDLE_SLEEP);
                Ok(false)
            }
        }
    }

    fn next_packet_size(&mut self) -> Result<Option<u32>, SourceError> {
        Ok(Some(match self.steps.front() {
            Some(Step::Packet { pcm, .. }) => (pcm.len() / CHANNELS) as u32,
            Some(Step::ReadFails(_)) => 1,
            _ => 0,
        }))
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<(u32, PacketInfo), SourceError> {
        match self.steps.pop_front() {
            Some(Step::Packet { pcm, info }) => {
                for (chunk, sample) in buffer.chunks_exact_mut(4).zip(&pcm) {
                    chunk.copy_from_slice(&sample.to_le_bytes());
                }
                Ok(((pcm.len() / CHANNELS) as u32, info))
            }
            Some(Step::ReadFails(error)) => Err(error),
            _ => Ok((0, PacketInfo::default())),
        }
    }

    fn stop(&mut self) {
        self.stopped.store(true, Ordering::Release);
    }
}
