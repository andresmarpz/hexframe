use greyscreen_core::encoder::{self, EncoderKind};
use greyscreen_core::media::{CaptureConfig, CaptureTarget};

use super::{encode_frames, primary_size};
use crate::adapters;
use crate::com::{ComGuard, MfGuard};
use crate::h264::{EncodedVideo, PlaneMeans};

const DEEP_FRAMES: i64 = 300;

#[derive(Debug)]
pub struct DeepRun {
    pub encoder: String,
    pub properties: Vec<String>,
    pub cpu_readback: bool,
    pub size: (u32, u32),
    pub fps: u32,
    pub frames_in: i64,
    pub packets: Vec<EncodedVideo>,
    pub nv12: Option<PlaneMeans>,
}

#[derive(Debug)]
pub struct DeepAdapter {
    pub kind: EncoderKind,
    pub adapter: String,
    pub vendor_id: u32,
    pub luid: u64,
    pub driver_version: String,
    pub is_capture_adapter: bool,
    pub run: Result<DeepRun, String>,
}

pub fn deep(cfg: &CaptureConfig) -> Vec<DeepAdapter> {
    let guards = ComGuard::mta().and_then(|com| Ok((com, MfGuard::start()?)));
    let adapters = adapters::for_target(&CaptureTarget::PrimaryMonitor);
    let native = primary_size();
    let results = encoder::plan(&adapters)
        .into_iter()
        .map(|candidate| {
            let adapter = adapters.iter().find(|a| a.luid == candidate.adapter);
            let run = match &guards {
                Ok(_) => encode_frames(candidate, cfg, native, DEEP_FRAMES),
                Err(error) => Err(format!("COM or Media Foundation failed: {error}")),
            };
            DeepAdapter {
                kind: candidate.kind,
                adapter: adapter.map_or_else(String::new, |a| a.name.clone()),
                vendor_id: adapter.map_or(0, |a| a.vendor_id),
                luid: candidate.adapter,
                driver_version: adapter.map_or_else(String::new, |a| a.driver_version.clone()),
                is_capture_adapter: adapter.is_some_and(|a| a.display),
                run,
            }
        })
        .collect();
    drop(guards);
    results
}
