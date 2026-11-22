use serde::{Deserialize, Serialize};

use crate::media::CaptureConfig;

mod check;
pub use check::{Attempt, CheckOutcome, EncoderCheck};

pub const SOFTWARE_MAX_HEIGHT: u32 = 1080;
pub const SOFTWARE_MAX_FPS: u32 = 30;
const MIN_WORKER_THREADS: u32 = 2;
const MAX_WORKER_THREADS: u32 = 4;
pub const SOFTWARE_LABEL: &str = "Windows software encoder";
pub const SOFTWARE_NOTICE: &str = "Your graphics card's video encoder didn't work, so Greyscreen records with the Windows software encoder. It uses more CPU and records at up to 1080p 30 fps.";
pub const FAILED_TITLE: &str = "No video encoder works on this PC";
pub const FAILED_HELP: &str = "Greyscreen can't record games until one works. Update your graphics driver from NVIDIA, AMD or Intel, restart your PC, then open Greyscreen again.";
pub const BUSY_HELP: &str = "Another app is using the video encoder. Close OBS, other recorders and video editors, then try again.";
const ENCODER_STOPPED: &str = "the video encoder stopped";
const DEVICE_LOST: &str = "graphics device";

const VENDOR_NVIDIA: u32 = 0x10de;
const VENDOR_AMD: u32 = 0x1002;
const VENDOR_AMD_ALT: u32 = 0x1022;
const VENDOR_INTEL: u32 = 0x8086;
const VENDOR_QUALCOMM: u32 = 0x5143;
const VENDOR_MICROSOFT: u32 = 0x1414;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncoderKind {
    Hardware,
    Software,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Adapter {
    pub luid: u64,
    pub name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub subsystem_id: u32,
    pub revision: u32,
    pub driver_version: String,
    pub display: bool,
    pub high_performance: bool,
    pub power_saving: bool,
}

impl Adapter {
    pub fn is_software(&self) -> bool {
        self.vendor_id == VENDOR_MICROSOFT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub adapter: u64,
    pub kind: EncoderKind,
}

pub fn plan(adapters: &[Adapter]) -> Vec<Candidate> {
    let mut sorted: Vec<&Adapter> = adapters.iter().collect();
    sorted.sort_by_key(|a| a.luid);
    let hardware: Vec<&Adapter> = sorted
        .iter()
        .filter(|a| !a.is_software())
        .copied()
        .collect();
    let display = hardware.iter().find(|a| a.display).copied();
    let mut order: Vec<u64> = Vec::new();
    let mut push = |luid: u64| {
        if !order.contains(&luid) {
            order.push(luid);
        }
    };
    if let Some(gpu) = game_gpu(&hardware, display) {
        push(gpu.luid);
    }
    if let Some(display) = display {
        push(display.luid);
    }
    for adapter in &hardware {
        push(adapter.luid);
    }
    let mut candidates: Vec<Candidate> = order
        .into_iter()
        .map(|adapter| Candidate {
            adapter,
            kind: EncoderKind::Hardware,
        })
        .collect();
    let software_adapter = display
        .or_else(|| sorted.iter().find(|a| a.display).copied())
        .or_else(|| sorted.first().copied());
    if let Some(adapter) = software_adapter {
        candidates.push(Candidate {
            adapter: adapter.luid,
            kind: EncoderKind::Software,
        });
    }
    candidates
}

// On a hybrid laptop the integrated GPU owns the display and games run on the discrete GPU.
fn game_gpu<'a>(hardware: &[&'a Adapter], display: Option<&Adapter>) -> Option<&'a Adapter> {
    let display = display?;
    if !display.power_saving || display.high_performance {
        return None;
    }
    hardware
        .iter()
        .find(|a| a.high_performance && a.luid != display.luid)
        .copied()
}

pub fn fingerprint(adapters: &[Adapter]) -> String {
    adapters
        .iter()
        .map(|a| {
            format!(
                "{} {:#06x} {:#06x} {:#010x} rev {} {}",
                a.name, a.vendor_id, a.device_id, a.subsystem_id, a.revision, a.driver_version
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn label(kind: EncoderKind, vendor_id: u32, adapter_name: &str) -> String {
    match kind {
        EncoderKind::Software => SOFTWARE_LABEL.to_owned(),
        EncoderKind::Hardware => {
            let vendor = match vendor_id {
                VENDOR_NVIDIA => "NVIDIA NVENC",
                VENDOR_AMD | VENDOR_AMD_ALT => "AMD AMF",
                VENDOR_INTEL => "Intel Quick Sync",
                VENDOR_QUALCOMM => "Qualcomm",
                _ => adapter_name,
            };
            format!("{vendor} (hardware)")
        }
    }
}

pub fn software_config(cfg: &CaptureConfig) -> CaptureConfig {
    let fps = cfg.fps.min(SOFTWARE_MAX_FPS);
    let max_height = cfg
        .max_height
        .map_or(SOFTWARE_MAX_HEIGHT, |h| h.min(SOFTWARE_MAX_HEIGHT));
    CaptureConfig {
        fps,
        gop_frames: cfg.gop_frames.min(fps),
        max_height: Some(max_height),
        ..cfg.clone()
    }
}

pub fn worker_threads(cpu_cores: u32) -> u32 {
    (cpu_cores / 2).clamp(MIN_WORKER_THREADS, MAX_WORKER_THREADS)
}

pub fn check_key(fingerprint: &str, cfg: &CaptureConfig, version: &str) -> String {
    format!(
        "{version} | {fingerprint} | {} fps, {} p, quality {}",
        cfg.fps,
        cfg.max_height
            .map_or("native".to_owned(), |h| h.to_string()),
        cfg.quality.map_or("vbr".to_owned(), |q| q.to_string()),
    )
}

pub fn stopped_reason(detail: &str) -> String {
    format!("{ENCODER_STOPPED}: {detail}")
}

pub fn is_encoder_failure(reason: &str) -> bool {
    reason.starts_with(ENCODER_STOPPED) || reason.starts_with(DEVICE_LOST)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Failed,
    Crashed,
    TimedOut,
    Busy,
}

impl Reason {
    pub fn is_unsure(self) -> bool {
        self != Reason::Failed
    }

    pub fn of_error(error: &str) -> Reason {
        if OpenFailure::from_text(error) == OpenFailure::Busy {
            Reason::Busy
        } else {
            Reason::Failed
        }
    }
}

pub const BUSY_CODES: &[u32] = &[];
const DEVICE_REMOVED_CODES: &[u32] = &[0x887A_0005, 0x887A_0006, 0x887A_0007];
const FAILURE_TAG: &str = "[open failure: ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenStep {
    Setup,
    Activate,
    D3dManager,
    OutputType,
    InputType,
    Start,
}

impl OpenStep {
    fn fallback(self) -> OpenFailure {
        match self {
            OpenStep::Setup | OpenStep::Activate | OpenStep::OutputType => OpenFailure::Other,
            OpenStep::D3dManager => OpenFailure::Unsupported,
            OpenStep::InputType => OpenFailure::InputRefused,
            OpenStep::Start => OpenFailure::StartFailed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFailure {
    Busy,
    StartFailed,
    DeviceRemoved,
    Unsupported,
    InputRefused,
    Other,
}

impl OpenFailure {
    pub const ALL: [OpenFailure; 6] = [
        OpenFailure::Busy,
        OpenFailure::StartFailed,
        OpenFailure::DeviceRemoved,
        OpenFailure::Unsupported,
        OpenFailure::InputRefused,
        OpenFailure::Other,
    ];

    pub fn at_step(step: OpenStep, vendor_id: u32, code: u32) -> OpenFailure {
        Self::at_step_with(step, vendor_id, code, BUSY_CODES)
    }

    pub fn at_step_with(step: OpenStep, vendor_id: u32, code: u32, busy: &[u32]) -> OpenFailure {
        if vendor_id == VENDOR_NVIDIA && busy.contains(&code) {
            OpenFailure::Busy
        } else if DEVICE_REMOVED_CODES.contains(&code) {
            OpenFailure::DeviceRemoved
        } else {
            step.fallback()
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            OpenFailure::Busy => "busy",
            OpenFailure::StartFailed => "start failed",
            OpenFailure::DeviceRemoved => "device removed",
            OpenFailure::Unsupported => "unsupported",
            OpenFailure::InputRefused => "input refused",
            OpenFailure::Other => "other",
        }
    }

    pub fn tag(self, detail: &str) -> String {
        format!("{detail} {FAILURE_TAG}{}]", self.name())
    }

    pub fn from_text(text: &str) -> OpenFailure {
        let found: Vec<OpenFailure> = text
            .split(FAILURE_TAG)
            .skip(1)
            .filter_map(|rest| {
                let name = rest.split(']').next()?;
                Self::ALL.into_iter().find(|f| f.name() == name)
            })
            .collect();
        if found.contains(&OpenFailure::Busy) {
            OpenFailure::Busy
        } else {
            found.first().copied().unwrap_or(OpenFailure::Other)
        }
    }

    pub fn is_busy_on(self, vendor_id: u32) -> bool {
        self == OpenFailure::Busy && vendor_id == VENDOR_NVIDIA
    }

    pub fn help(self, vendor_id: u32) -> &'static str {
        if self.is_busy_on(vendor_id) {
            BUSY_HELP
        } else {
            FAILED_HELP
        }
    }
}

pub fn fallback_warning(kind: EncoderKind, failed: &[(OpenFailure, u32)]) -> Option<String> {
    if kind != EncoderKind::Software || failed.is_empty() {
        return None;
    }
    let busy = failed.iter().any(|(f, vendor)| f.is_busy_on(*vendor));
    Some(if busy { BUSY_HELP } else { SOFTWARE_NOTICE }.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    pub kind: EncoderKind,
    pub adapter: String,
    pub vendor_id: u32,
    pub driver_version: String,
    #[serde(skip)]
    pub luid: u64,
}

impl Probe {
    pub fn same_target(&self, other: &Probe) -> bool {
        self.kind == other.kind
            && self.adapter == other.adapter
            && self.driver_version == other.driver_version
    }
}

pub const PROBE_FLAG: &str = "--greyscreen-probe";
pub const PROBE_CRASH_FLAG: &str = "--crash";
pub const PROBE_SELF_DEADLINE_MS: u64 = 9000;
const PROBE_PREFIX: &str = "result=";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeRequest {
    pub luid: u64,
    pub kind: EncoderKind,
    pub cfg: CaptureConfig,
}

impl ProbeRequest {
    pub fn to_arg(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_arg(arg: &str) -> Option<Self> {
        serde_json::from_str(arg).ok()
    }

    pub fn probe(&self) -> Probe {
        Probe {
            kind: self.kind,
            adapter: String::new(),
            vendor_id: 0,
            driver_version: String::new(),
            luid: self.luid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeLine {
    Passed(String),
    Failed(String),
    TimedOut(String),
}

impl ProbeLine {
    pub fn format(&self) -> String {
        let (status, detail) = match self {
            Self::Passed(d) => ("ok", d),
            Self::Failed(d) => ("failed", d),
            Self::TimedOut(d) => ("timeout", d),
        };
        let detail = detail.replace(['\r', '\n'], " ");
        format!("{PROBE_PREFIX}{status} detail={detail}")
    }

    pub fn parse(output: &str) -> Option<Self> {
        let line = output
            .lines()
            .rev()
            .find_map(|l| l.trim().strip_prefix(PROBE_PREFIX))?;
        let (status, rest) = line.split_once(' ').unwrap_or((line, ""));
        let detail = rest.strip_prefix("detail=").unwrap_or(rest).to_owned();
        match status {
            "ok" => Some(Self::Passed(detail)),
            "failed" => Some(Self::Failed(detail)),
            "timeout" => Some(Self::TimedOut(detail)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub probe: Probe,
    pub reason: Option<Reason>,
}

#[cfg(test)]
mod tests;
