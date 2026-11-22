use std::sync::atomic::Ordering;
use std::time::Instant;

use greyscreen_core::encoder;
use greyscreen_core::frame_timing::{Pacer, Watchdog};
use greyscreen_core::time::mono_now;
use windows::Win32::Graphics::Dxgi::{DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET};
use windows::core::HRESULT;

use super::Flags;
use super::audio::AudioPipe;
use super::emit::Sink;
use crate::client_area::ClientArea;
use crate::com::{ComGuard, MfGuard};
use crate::d3d::GpuDevice;
use crate::h264::{EncodedVideo, H264Encoder, H264Error};
use crate::pacer::{Stats, wait_for};
use crate::power::SystemPowerGuard;
use crate::procstats::ProcessSampler;
use crate::vproc::{VideoConverter, VprocError};
use crate::wgc::WgcCapture;

#[cfg(test)]
mod tests;

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
    pub(super) client_area: ClientArea,
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
    pub(super) sampler: Option<ProcessSampler>,
    pub(super) seq: u64,
    pub(super) last_seq: u64,
    pub(super) soft_errors: u32,
    pub(super) power: SystemPowerGuard,
}

impl Video {
    pub(crate) fn run(mut self) {
        let exit = self.drive();
        self.shutdown(exit);
    }

    fn drive(&mut self) -> Exit {
        loop {
            if !wait_for(&self.pacer, &self.flags.stop) {
                return Exit::Stopped;
            }
            let before = self.pacer.tick();
            if self.pacer.resync(mono_now()) {
                self.stats.c.resyncs += 1;
                self.stats.c.skipped += self.pacer.tick() - before;
            }
            let started = Instant::now();
            if let Err(exit) = self.step() {
                return exit;
            }
            self.stats.step_done(started);
            self.pacer.advance();
            self.stats.c.ticks += 1;
            self.stats.report_if_due(self.sink.encoded());
            if self.stats.snapshot_due() {
                self.publish_stats();
            }
        }
    }

    fn step(&mut self) -> Result<(), Exit> {
        // SAFETY: the device is live and the call only reads its state.
        if let Err(error) = fault!(DeviceRemovedCheck)
            .and_then(|()| unsafe { self.gpu.device.GetDeviceRemovedReason() })
        {
            return Err(Exit::Lost(format!("graphics device removed: {error}")));
        }
        self.drain_video()?;
        let submitting = self.process_frame()?;
        self.drain_video()?;
        if let Some(reason) = self.sink.take_fault() {
            return Err(Exit::Failed(reason));
        }
        if self
            .watchdog
            .stalled(mono_now(), self.sink.encoded(), submitting)
        {
            return Err(Exit::Failed(encoder::stopped_reason(
                "no frames came out for too long",
            )));
        }
        if let Some(audio) = &self.audio {
            self.stats.c.audio_packets += audio.drain(&mut self.sink);
        }
        if self.wgc.is_closed() {
            return Err(Exit::Closed);
        }
        if self.sink.is_closed() {
            return Err(Exit::Stopped);
        }
        Ok(())
    }

    fn publish_stats(&mut self) {
        if let Some(audio) = &self.audio {
            audio.fill_stats(&mut self.stats.c);
        }
        self.sink.fill_stats(&mut self.stats.c);
        self.wgc.fill_stats(&mut self.stats.c);
        if let Some(sampler) = &mut self.sampler {
            let (cpu, mem) = sampler.sample();
            self.stats.c.cpu_pct = cpu;
            self.stats.c.mem_mb = mem;
            self.stats.c.foreground = sampler.target_in_foreground();
            sampler.fill_gpu(&mut self.stats.c);
        }
        let snapshot = self.stats.take_snapshot(mono_now());
        self.sink.stats(snapshot);
    }

    fn process_frame(&mut self) -> Result<bool, Exit> {
        if !self.wgc.has_frame() {
            return Ok(false);
        }
        match self.wait_for_input() {
            Ok(true) => {}
            Ok(false) => {
                self.stats.c.dropped += 1;
                return Ok(true);
            }
            Err(error) => return self.encoder_error("encoder poll", &error).map(|()| true),
        }
        let Some((texture, size, seq)) = self.wgc.latest() else {
            return Ok(false);
        };
        self.seq = seq;
        self.stats.c.captured = seq;
        let crop = self.client_area.crop(size);
        let converted = self.converter.convert(&texture, size, crop);
        self.wgc.done_reading();
        let frame = match converted {
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
            return self.encoder_error("encoder submit", &error).map(|()| true);
        }
        self.sink.submitted(self.pacer.tick(), pts);
        if seq == self.last_seq {
            self.stats.c.repeated += 1;
        }
        self.last_seq = seq;
        self.soft_errors = 0;
        Ok(true)
    }

    fn wait_for_input(&mut self) -> Result<bool, H264Error> {
        let deadline = mono_now() + self.pacer.frame_hns() / 2;
        loop {
            if self.encoder.need_input()? {
                return Ok(true);
            }
            if mono_now() >= deadline {
                return Ok(false);
            }
            self.encoder.drain_output(&mut self.scratch)?;
            self.sink.video(&mut self.scratch);
            std::thread::sleep(std::time::Duration::from_micros(250));
        }
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
            return self.encoder_error("encoder drain", &error);
        }
        self.sink.video(&mut self.scratch);
        Ok(())
    }

    fn encoder_error(&mut self, what: &str, error: &H264Error) -> Result<(), Exit> {
        if let H264Error::Stopped(code) = error {
            return Err(Exit::Failed(encoder::stopped_reason(&code.to_string())));
        }
        self.soft(what, error, lost_h264(error))
    }

    fn soft(&mut self, what: &str, error: &dyn std::fmt::Display, lost: bool) -> Result<(), Exit> {
        if lost {
            return Err(Exit::Lost(format!(
                "graphics device lost in {what}: {error}"
            )));
        }
        tracing::warn!(%error, what, "capture step failed");
        self.stats.c.dropped += 1;
        self.stats.c.soft_errors += 1;
        self.soft_errors += 1;
        if self.soft_errors >= MAX_SOFT_ERRORS {
            let reason = format!("{what} keeps failing: {error}");
            return Err(Exit::Failed(if what.starts_with("encoder") {
                encoder::stopped_reason(&reason)
            } else {
                reason
            }));
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
            mut power,
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
        power.release();
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
