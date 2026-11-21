use std::sync::atomic::Ordering;

use greyscreen_core::time::mono_now;
use windows::Win32::Graphics::Dxgi::{DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET};
use windows::core::HRESULT;

use super::Flags;
use super::audio::AudioPipe;
use super::emit::Sink;
use crate::com::{ComGuard, MfGuard};
use crate::d3d::GpuDevice;
use crate::h264::{EncodedVideo, H264Encoder, H264Error};
use crate::pacer::{Pacer, Stats};
use crate::vproc::{VideoConverter, VprocError};
use crate::watchdog::Watchdog;
use crate::wgc::WgcCapture;

const MAX_SOFT_ERRORS: u32 = 120;

enum Exit {
    Stopped,
    Closed,
    Lost(String),
    Failed(String),
}

pub(crate) struct Video {
    pub(super) wgc: WgcCapture,
    pub(super) converter: VideoConverter,
    pub(super) encoder: H264Encoder,
    pub(super) audio: Option<AudioPipe>,
    pub(super) sink: Sink,
    pub(super) pacer: Pacer,
    pub(super) watchdog: Watchdog,
    pub(super) flags: Flags,
    pub(super) gpu: GpuDevice,
    pub(super) mf: MfGuard,
    pub(super) com: ComGuard,
    pub(super) scratch: Vec<EncodedVideo>,
    pub(super) stats: Stats,
    pub(super) seq: u64,
    pub(super) soft_errors: u32,
}

impl Video {
    pub(crate) fn run(mut self) {
        let exit = self.drive();
        self.shutdown(exit);
    }

    fn drive(&mut self) -> Exit {
        loop {
            if !self.pacer.wait(&self.flags.stop) {
                return Exit::Stopped;
            }
            if self.pacer.resync(mono_now()) {
                self.stats.skips += 1;
            }
            if let Err(exit) = self.step() {
                return exit;
            }
            self.pacer.advance();
            self.stats.report_if_due(self.seq, self.sink.encoded());
        }
    }

    fn step(&mut self) -> Result<(), Exit> {
        // SAFETY: the device is live and the call only reads its state.
        if let Err(error) = unsafe { self.gpu.device.GetDeviceRemovedReason() } {
            return Err(Exit::Lost(format!("graphics device removed: {error}")));
        }
        let submitting = self.process_frame()?;
        self.drain_video()?;
        if let Some(reason) = self.sink.take_fault() {
            return Err(Exit::Failed(reason));
        }
        if self
            .watchdog
            .stalled(mono_now(), self.sink.encoded(), submitting)
        {
            return Err(Exit::Failed("encoder stopped producing frames".into()));
        }
        if let Some(audio) = &self.audio {
            self.stats.audio += audio.drain(&mut self.sink);
        }
        if self.wgc.is_closed() {
            return Err(Exit::Closed);
        }
        if self.sink.is_closed() {
            return Err(Exit::Stopped);
        }
        Ok(())
    }

    fn process_frame(&mut self) -> Result<bool, Exit> {
        let Some((texture, size, seq)) = self.wgc.latest() else {
            return Ok(false);
        };
        self.seq = seq;
        match self.encoder.need_input() {
            Ok(true) => {}
            Ok(false) => {
                self.stats.dropped += 1;
                return Ok(true);
            }
            Err(error) => return self.soft_frame("encoder poll", &error, lost_h264(&error)),
        }
        let frame = match self.converter.convert(&texture, size) {
            Ok(frame) => frame,
            Err(VprocError::Busy) => {
                self.stats.busy += 1;
                return Ok(true);
            }
            Err(error) => return self.soft_frame("convert", &error, lost_vproc(&error)),
        };
        if self.flags.keyframe.swap(false, Ordering::AcqRel)
            && let Err(error) = self.encoder.force_keyframe()
        {
            tracing::warn!(%error, "forcing a key frame failed");
        }
        let (pts, dur) = (self.pacer.pts(), self.pacer.frame_hns());
        if let Err(error) = self.encoder.submit(&frame, pts, dur) {
            self.converter.release(frame.slot);
            return self.soft_frame("encoder submit", &error, lost_h264(&error));
        }
        self.sink.submitted(self.pacer.tick(), pts);
        self.soft_errors = 0;
        Ok(true)
    }

    fn soft_frame(
        &mut self,
        what: &str,
        error: &dyn std::fmt::Display,
        lost: bool,
    ) -> Result<bool, Exit> {
        self.soft(what, error, lost).map(|()| true)
    }

    fn drain_video(&mut self) -> Result<(), Exit> {
        if let Err(error) = self.encoder.drain_output(&mut self.scratch) {
            self.sink.video(&mut self.scratch);
            return self.soft("encoder drain", &error, lost_h264(&error));
        }
        self.sink.video(&mut self.scratch);
        Ok(())
    }

    fn soft(&mut self, what: &str, error: &dyn std::fmt::Display, lost: bool) -> Result<(), Exit> {
        if lost {
            return Err(Exit::Lost(format!(
                "graphics device lost in {what}: {error}"
            )));
        }
        tracing::warn!(%error, what, "capture step failed");
        self.stats.dropped += 1;
        self.soft_errors += 1;
        if self.soft_errors >= MAX_SOFT_ERRORS {
            return Err(Exit::Failed(format!("{what} keeps failing: {error}")));
        }
        Ok(())
    }

    fn shutdown(self, exit: Exit) {
        let Self {
            wgc,
            converter,
            encoder,
            audio,
            mut sink,
            mut scratch,
            gpu,
            mf,
            com,
            ..
        } = self;
        let (finish, failure) = match exit {
            Exit::Stopped => (true, None),
            Exit::Closed => (true, Some("the captured window closed".to_string())),
            Exit::Lost(reason) | Exit::Failed(reason) => (false, Some(reason)),
        };
        if finish {
            if let Err(error) = encoder.finish(&mut scratch) {
                tracing::warn!(%error, "encoder finish failed");
            }
            sink.video(&mut scratch);
        } else {
            drop(encoder);
        }
        if let Some(audio) = audio {
            audio.finish(&mut sink);
        }
        wgc.stop();
        drop(converter);
        drop(gpu);
        drop(mf);
        drop(com);
        if let Some(reason) = failure {
            sink.failed(reason);
        }
    }
}

fn is_device_lost(code: HRESULT) -> bool {
    code == DXGI_ERROR_DEVICE_REMOVED || code == DXGI_ERROR_DEVICE_RESET
}

fn lost_h264(error: &H264Error) -> bool {
    matches!(error, H264Error::Windows(e) if is_device_lost(e.code()))
}

fn lost_vproc(error: &VprocError) -> bool {
    matches!(error, VprocError::Windows(e) if is_device_lost(e.code()))
}
