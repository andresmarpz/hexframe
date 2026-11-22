use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::aac::{AacError, OpenEncoder, PcmEncoder, Result};
use crate::audio::PcmBlock;

pub(crate) type Received = Arc<Mutex<Vec<PcmBlock>>>;

pub(crate) struct FakeEncoder {
    fail_at: Option<usize>,
    delay: Duration,
    flush_frames: usize,
    blocks: usize,
    received: Received,
}

impl FakeEncoder {
    pub(crate) fn new() -> Self {
        Self {
            fail_at: None,
            delay: Duration::ZERO,
            flush_frames: 0,
            blocks: 0,
            received: Received::default(),
        }
    }

    pub(crate) fn failing_at(mut self, block: usize) -> Self {
        self.fail_at = Some(block);
        self
    }

    pub(crate) fn sleeping(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub(crate) fn flushing(mut self, frames: usize) -> Self {
        self.flush_frames = frames;
        self
    }

    pub(crate) fn received(&self) -> Received {
        Arc::clone(&self.received)
    }

    pub(crate) fn open(self) -> OpenEncoder {
        Box::new(move || Ok(Box::new(self)))
    }
}

pub(crate) fn failing_open() -> OpenEncoder {
    Box::new(|| Err(AacError::Unsupported("scripted open failure")))
}

impl PcmEncoder for FakeEncoder {
    fn encode(&mut self, block: &PcmBlock, out: &mut Vec<Vec<u8>>) -> Result<()> {
        std::thread::sleep(self.delay);
        let index = self.blocks;
        self.blocks += 1;
        if self.fail_at == Some(index) {
            return Err(AacError::Unsupported("scripted encode failure"));
        }
        self.received
            .lock()
            .expect("received lock")
            .push(block.clone());
        out.push(vec![index as u8]);
        Ok(())
    }

    fn finish(self: Box<Self>, out: &mut Vec<Vec<u8>>) -> Result<()> {
        out.extend((0..self.flush_frames).map(|index| vec![index as u8]));
        Ok(())
    }

    fn asc(&self) -> Vec<u8> {
        vec![0x11, 0x90]
    }
}
