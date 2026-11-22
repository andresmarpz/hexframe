use greyscreen_core::encoder::{Nv12Means, Nv12Report, nv12_report};
use greyscreen_core::frame_timing::OrderGuard;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Packet {
    pub(crate) annexb: Vec<u8>,
    pub(crate) pts_hns: i64,
    pub(crate) key: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Observation {
    pub(crate) adapter: String,
    pub(crate) vendor_id: u32,
    pub(crate) luid: u64,
    pub(crate) driver_version: String,
    pub(crate) hardware: bool,
    pub(crate) is_capture_adapter: bool,
    pub(crate) encoder: String,
    pub(crate) properties: Vec<String>,
    pub(crate) cpu_readback: bool,
    pub(crate) size: (u32, u32),
    pub(crate) fps: u32,
    pub(crate) frames_in: i64,
    pub(crate) packets: Vec<Packet>,
    pub nv12: Option<Nv12Means>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub(crate) greyscreen: String,
    pub(crate) os: String,
    pub(crate) pass: bool,
    pub(crate) open_bframe_bug: bool,
    pub(crate) adapters: Vec<AdapterReport>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AdapterReport {
    pub(crate) adapter: String,
    pub(crate) vendor_id: u32,
    pub(crate) luid: String,
    pub(crate) driver_version: String,
    pub(crate) kind: &'static str,
    pub(crate) encoder: String,
    pub(crate) pass: bool,
    pub(crate) failures: Vec<String>,
    pub(crate) notes: Vec<String>,
    pub(crate) frames_in: i64,
    pub(crate) packets_out: usize,
    pub(crate) b_frames: u32,
    pub(crate) pts_strictly_rising: bool,
    pub(crate) step_backs: u64,
    pub(crate) encodes_on_capture_adapter: bool,
    pub(crate) cpu_readback: bool,
    pub(crate) size: String,
    pub(crate) fps: u32,
    pub(crate) properties: Vec<String>,
    pub nv12: Nv12Report,
}

pub(crate) fn build(observations: &[Observation], greyscreen: &str, os: &str) -> Report {
    let adapters: Vec<AdapterReport> = observations.iter().map(judge).collect();
    Report {
        greyscreen: greyscreen.to_owned(),
        os: os.to_owned(),
        pass: !adapters.is_empty() && adapters.iter().all(|a| a.pass),
        open_bframe_bug: adapters
            .iter()
            .any(|a| a.b_frames > 0 || !a.pts_strictly_rising && a.packets_out > 0),
        adapters,
    }
}

fn judge(observation: &Observation) -> AdapterReport {
    let packets = &observation.packets;
    let b_frames = packets.iter().map(|p| count_b_pictures(&p.annexb)).sum();
    let mut order = OrderGuard::new();
    for packet in packets {
        let _stepped_back = order.check(packet.pts_hns).is_err();
    }
    let step_backs = order.step_backs();
    let rising = step_backs == 0;
    let mut failures = Vec::new();
    let mut notes = Vec::new();
    if let Some(error) = &observation.error {
        failures.push(format!("encode failed: {error}"));
    } else {
        if packets.is_empty() {
            failures.push("the encoder produced no packets".to_owned());
        }
        if packets.first().is_some_and(|p| !p.key) {
            failures.push("the first packet is not a key frame".to_owned());
        }
        if b_frames > 0 {
            failures.push(format!("{b_frames} B-frames read back with B-frames off"));
        }
        if !rising {
            failures.push(format!("output timestamps stepped back {step_backs} times"));
        }
        if observation.hardware && observation.cpu_readback {
            failures.push("the hardware path made a CPU readback".to_owned());
        }
        if observation.hardware && !observation.is_capture_adapter {
            notes.push("this encoder is not on the capture adapter".to_owned());
        }
        if (packets.len() as i64) < observation.frames_in {
            notes.push(format!(
                "{} packets for {} frames",
                packets.len(),
                observation.frames_in
            ));
        }
    }
    AdapterReport {
        adapter: observation.adapter.clone(),
        vendor_id: observation.vendor_id,
        luid: format!("{:016x}", observation.luid),
        driver_version: observation.driver_version.clone(),
        kind: if observation.hardware {
            "hardware"
        } else {
            "software"
        },
        encoder: observation.encoder.clone(),
        pass: failures.is_empty(),
        failures,
        notes,
        frames_in: observation.frames_in,
        packets_out: packets.len(),
        b_frames,
        pts_strictly_rising: rising,
        step_backs,
        encodes_on_capture_adapter: observation.is_capture_adapter,
        cpu_readback: observation.cpu_readback,
        size: format!("{}x{}", observation.size.0, observation.size.1),
        fps: observation.fps,
        properties: observation.properties.clone(),
        nv12: nv12_report(observation.nv12),
    }
}

pub(crate) fn count_b_pictures(annexb: &[u8]) -> u32 {
    let mut count = 0;
    let mut index = 0;
    while let Some(offset) = annexb[index..].windows(3).position(|w| w == [0, 0, 1]) {
        let start = index + offset + 3;
        index = start;
        let Some(&header) = annexb.get(start) else {
            break;
        };
        if matches!(header & 0x1f, 1 | 5) && slice_is_b(&annexb[start + 1..]) {
            count += 1;
        }
    }
    count
}

fn slice_is_b(payload: &[u8]) -> bool {
    let head: Vec<u8> = payload.iter().copied().take(16).collect();
    let rbsp = strip_emulation_prevention(&head);
    let mut bits = Bits {
        data: &rbsp,
        pos: 0,
    };
    let (Some(first_mb), Some(slice_type)) = (bits.ue(), bits.ue()) else {
        return false;
    };
    first_mb == 0 && slice_type % 5 == 1
}

fn strip_emulation_prevention(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut zeros = 0;
    for &byte in data {
        if zeros >= 2 && byte == 3 {
            zeros = 0;
            continue;
        }
        zeros = if byte == 0 { zeros + 1 } else { 0 };
        out.push(byte);
    }
    out
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.pos / 8)?;
        let bit = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Some(u32::from(bit))
    }

    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        let mut value = 0u32;
        for _ in 0..zeros {
            value = (value << 1) | self.bit()?;
        }
        Some((1u32 << zeros) - 1 + value)
    }
}
