use windows::Graphics::Capture::Direct3D11CaptureFramePool;

use super::{Ordering, PublishProbe, QpcMap, Result, WgcCapture, WgcOptions};
use crate::d3d::GpuDevice;
use greyscreen_core::media::CaptureTarget;

impl PublishProbe {
    pub fn deliver_frame(&self, pool: &Direct3D11CaptureFramePool) -> windows::core::Result<bool> {
        self.0.on_frame(pool)
    }

    pub fn frames_counted(&self) -> u64 {
        self.0.frames.load(Ordering::Relaxed)
    }
}

impl WgcCapture {
    pub fn start_without_session(
        gpu: &GpuDevice,
        target: &CaptureTarget,
        qpc: QpcMap,
        options: WgcOptions,
    ) -> Result<Self> {
        Self::unattached(gpu, target, qpc, options)
    }

    pub fn frame_pool(&self) -> Option<Direct3D11CaptureFramePool> {
        self.pool.clone()
    }
}
