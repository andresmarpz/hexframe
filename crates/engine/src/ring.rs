use std::collections::VecDeque;
use std::sync::Arc;

use greyscreen_core::media::{Packet, Track, VideoFormat};
use greyscreen_core::time::{HNS_PER_SEC, Hns};

#[derive(Debug)]
pub(crate) struct ClipRing {
    packets: VecDeque<Arc<Packet>>,
    keep: Hns,
    t0: Hns,
    audio_rate: i64,
    newest: Hns,
}

impl ClipRing {
    pub(crate) fn new(t0: Hns, keep: Hns, audio_rate: u32) -> Self {
        Self {
            packets: VecDeque::new(),
            keep,
            t0,
            audio_rate: i64::from(audio_rate.max(1)),
            newest: t0,
        }
    }

    pub(crate) fn push(&mut self, packet: Arc<Packet>) {
        self.newest = self.newest.max(self.start_of(&packet));
        self.packets.push_back(packet);
    }

    pub(crate) fn prune(&mut self) {
        let cutoff = self.newest - self.keep;
        let Some(idr) = self
            .packets
            .iter()
            .rev()
            .find(|p| is_idr(p) && self.start_of(p) <= cutoff)
            .map(|p| self.start_of(p))
        else {
            return;
        };
        while let Some(front) = self.packets.front() {
            let before = match front.track {
                Track::Video => self.start_of(front) < idr,
                Track::Audio => self.end_of(front) <= idr,
            };
            if !before {
                break;
            }
            self.packets.pop_front();
        }
    }

    pub(crate) fn range(&self, start: Hns, end: Hns) -> Vec<Arc<Packet>> {
        let idrs = || self.packets.iter().filter(|p| is_idr(p));
        let first = idrs()
            .map(|p| self.start_of(p))
            .filter(|t| *t <= start)
            .max()
            .or_else(|| idrs().map(|p| self.start_of(p)).min());
        let Some(first) = first else {
            return Vec::new();
        };
        self.packets
            .iter()
            .filter(|p| match p.track {
                Track::Video => self.start_of(p) >= first && self.start_of(p) < end,
                Track::Audio => self.end_of(p) > first && self.start_of(p) < end,
            })
            .cloned()
            .collect()
    }

    fn scale(&self, packet: &Packet) -> i64 {
        match packet.track {
            Track::Video => i64::from(VideoFormat::TIMESCALE),
            Track::Audio => self.audio_rate,
        }
    }

    fn start_of(&self, packet: &Packet) -> Hns {
        self.t0 + packet.dts * HNS_PER_SEC / self.scale(packet)
    }

    fn end_of(&self, packet: &Packet) -> Hns {
        self.start_of(packet) + i64::from(packet.duration) * HNS_PER_SEC / self.scale(packet)
    }
}

fn is_idr(packet: &Packet) -> bool {
    packet.track == Track::Video && packet.key
}

#[cfg(test)]
mod tests;
