use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use crossbeam_channel::bounded;
use greyscreen_core::media::{
    AudioFormat, CaptureConfig, CaptureTarget, MediaEvent, MediaSource, Packet, Track, VideoFormat,
};
use greyscreen_core::settings::Settings;
use greyscreen_engine::FakeMediaSource;

const EVENT_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) struct Media {
    pub(super) video: VideoFormat,
    pub(super) audio: AudioFormat,
    packets: Vec<Arc<Packet>>,
}

impl Media {
    pub(super) fn collect(secs: f64) -> Result<Media> {
        let (tx, rx) = bounded(1024);
        let cfg: CaptureConfig = Settings::default().capture_config(2160);
        let session = FakeMediaSource::fast()
            .start(CaptureTarget::PrimaryMonitor, cfg, tx)
            .map_err(|e| anyhow!("fixture source failed to start: {e}"))?;
        let mut formats = None;
        let mut packets = Vec::new();
        let outcome = loop {
            match rx.recv_timeout(EVENT_TIMEOUT) {
                Ok(MediaEvent::Started { video, audio, .. }) => formats = Some((video, audio)),
                Ok(MediaEvent::Packet(packet)) => {
                    let done = packet.track == Track::Video
                        && packet.dts as f64 / f64::from(VideoFormat::TIMESCALE) >= secs;
                    packets.push(Arc::new(packet));
                    if done {
                        break Ok(());
                    }
                }
                Ok(MediaEvent::Warning(_)) => {}
                Ok(MediaEvent::Failed(message)) => break Err(anyhow!(message)),
                Err(_) => break Err(anyhow!("fixture source went quiet")),
            }
        };
        drop(rx);
        session.stop();
        outcome?;
        let Some((video, Some(audio))) = formats else {
            bail!("fixture source gave no audio and video formats");
        };
        Ok(Media {
            video,
            audio,
            packets,
        })
    }

    fn rate(&self, packet: &Packet) -> f64 {
        match packet.track {
            Track::Video => f64::from(VideoFormat::TIMESCALE),
            Track::Audio => f64::from(self.audio.sample_rate),
        }
    }

    fn start_secs(&self, packet: &Packet) -> f64 {
        packet.dts as f64 / self.rate(packet)
    }

    fn end_secs(&self, packet: &Packet) -> f64 {
        self.start_secs(packet) + f64::from(packet.duration) / self.rate(packet)
    }

    pub(super) fn upto(&self, end: f64) -> Vec<Arc<Packet>> {
        self.range(0.0, end)
    }

    pub(super) fn clip(&self, start: f64, end: f64) -> Vec<Arc<Packet>> {
        let keyframe = self
            .packets
            .iter()
            .filter(|p| p.track == Track::Video && p.key)
            .map(|p| self.start_secs(p))
            .filter(|t| *t <= start)
            .fold(0.0, f64::max);
        self.range(keyframe, end)
    }

    fn range(&self, first: f64, end: f64) -> Vec<Arc<Packet>> {
        self.packets
            .iter()
            .filter(|p| match p.track {
                Track::Video => self.start_secs(p) >= first && self.start_secs(p) < end,
                Track::Audio => self.end_secs(p) > first && self.start_secs(p) < end,
            })
            .cloned()
            .collect()
    }
}
