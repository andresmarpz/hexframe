use serde::{Deserialize, Serialize};

use crate::media::CaptureConfig;

pub const SOFTWARE_MAX_HEIGHT: u32 = 1080;
pub const SOFTWARE_MAX_FPS: u32 = 30;
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
    let hardware: Vec<&Adapter> = adapters.iter().filter(|a| !a.is_software()).collect();
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
        .or_else(|| adapters.iter().find(|a| a.display))
        .or_else(|| adapters.first());
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
        .map(|a| format!("{} {:#06x} {}", a.name, a.vendor_id, a.driver_version))
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

    pub fn from_code(code: u32, fallback: OpenFailure) -> OpenFailure {
        Self::from_code_with(code, fallback, BUSY_CODES)
    }

    pub fn from_code_with(code: u32, fallback: OpenFailure, busy: &[u32]) -> OpenFailure {
        if busy.contains(&code) {
            OpenFailure::Busy
        } else if DEVICE_REMOVED_CODES.contains(&code) {
            OpenFailure::DeviceRemoved
        } else {
            fallback
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub kind: EncoderKind,
    pub adapter: String,
    pub vendor_id: u32,
    pub result: Result<String, String>,
    pub reason: Reason,
}

impl Attempt {
    pub fn label(&self) -> String {
        label(self.kind, self.vendor_id, &self.adapter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Hardware,
    Software,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderCheck {
    pub key: String,
    pub outcome: CheckOutcome,
    pub encoder: String,
    pub detail: String,
    #[serde(default)]
    pub noticed: bool,
    #[serde(default)]
    pub verdicts: Vec<Verdict>,
}

impl EncoderCheck {
    pub fn from_attempts(key: String, attempts: &[Attempt]) -> Self {
        let failures = attempts
            .iter()
            .filter_map(|a| {
                a.result
                    .as_ref()
                    .err()
                    .map(|e| format!("{}: {e}", a.label()))
            })
            .collect::<Vec<_>>()
            .join("; ");
        let chosen = attempts.iter().find(|a| a.result.is_ok());
        let (outcome, encoder) = match chosen {
            Some(a) if a.kind == EncoderKind::Hardware => (CheckOutcome::Hardware, a.label()),
            Some(a) => (CheckOutcome::Software, a.label()),
            None => (CheckOutcome::Failed, String::new()),
        };
        Self {
            key,
            outcome,
            encoder,
            detail: failures,
            noticed: false,
            verdicts: Vec::new(),
        }
    }

    pub fn reusable_for(&self, key: &str) -> bool {
        self.key == key
            && match self.outcome {
                CheckOutcome::Failed => false,
                CheckOutcome::Hardware => true,
                CheckOutcome::Software => !self.has_unsure_verdict(),
            }
    }

    pub fn has_unsure_verdict(&self) -> bool {
        self.verdicts
            .iter()
            .any(|v| v.reason.is_some_and(Reason::is_unsure))
    }

    pub fn crashed_before(&self, probe: &Probe) -> bool {
        self.verdicts
            .iter()
            .any(|v| v.reason == Some(Reason::Crashed) && v.probe.same_target(probe))
    }

    pub fn settings_line(&self) -> String {
        match self.outcome {
            CheckOutcome::Hardware => self.encoder.clone(),
            CheckOutcome::Software => format!("{} — uses more CPU", self.encoder),
            CheckOutcome::Failed => "None works".to_owned(),
        }
    }

    pub fn wants_notice(&self) -> bool {
        self.outcome == CheckOutcome::Software && !self.noticed
    }

    pub fn busy_encoder(&self) -> bool {
        self.verdicts
            .iter()
            .any(|v| v.reason == Some(Reason::Busy) && v.probe.vendor_id == VENDOR_NVIDIA)
    }

    pub fn notice_text(&self) -> &'static str {
        if self.busy_encoder() {
            BUSY_HELP
        } else {
            SOFTWARE_NOTICE
        }
    }

    pub fn failed_help(&self) -> &'static str {
        if self.busy_encoder() {
            BUSY_HELP
        } else {
            FAILED_HELP
        }
    }

    pub fn failed(&self) -> bool {
        self.outcome == CheckOutcome::Failed
    }
}

#[cfg(test)]
mod tests;
