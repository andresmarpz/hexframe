use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInstance<'a> {
    pub pid: u32,
    pub luid: u64,
    pub engine: &'a str,
    pub kind: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GpuUsage {
    pub busy_3d_pct: f32,
    pub self_3d_pct: f32,
    pub encode_pct: f32,
    pub copy_pct: f32,
}

pub fn parse_instance(name: &str) -> Option<EngineInstance<'_>> {
    let rest = name.strip_prefix("pid_")?;
    let (pid, rest) = rest.split_once("_luid_0x")?;
    let (high, rest) = rest.split_once("_0x")?;
    let (low, rest) = rest.split_once("_phys_")?;
    let (engine, kind) = rest.split_once("_engtype_")?;
    let luid = (u64::from_str_radix(high, 16).ok()? << 32) | u64::from_str_radix(low, 16).ok()?;
    if kind.is_empty() {
        return None;
    }
    Some(EngineInstance {
        pid: pid.parse().ok()?,
        luid,
        engine,
        kind,
    })
}

pub fn summarize<'a>(
    samples: impl IntoIterator<Item = (&'a str, f64)>,
    luid: Option<u64>,
    own_pid: u32,
) -> GpuUsage {
    let mut engines: HashMap<(u64, &str, &str), (f64, f64)> = HashMap::new();
    for (name, value) in samples {
        let Some(instance) = parse_instance(name) else {
            continue;
        };
        if luid.is_some_and(|wanted| wanted != instance.luid) {
            continue;
        }
        let value = value.max(0.0);
        let slot = engines
            .entry((instance.luid, instance.engine, instance.kind))
            .or_default();
        slot.0 += value;
        if instance.pid == own_pid {
            slot.1 += value;
        }
    }
    let mut usage = GpuUsage::default();
    let mut copy_total = 0.0f64;
    for ((_, _, kind), (total, own)) in &engines {
        match *kind {
            "3D" => {
                usage.busy_3d_pct = usage.busy_3d_pct.max(*total as f32);
                usage.self_3d_pct = usage.self_3d_pct.max(*own as f32);
            }
            "VideoEncode" => usage.encode_pct = usage.encode_pct.max(*total as f32),
            "Copy" => copy_total += total,
            _ => {}
        }
    }
    usage.busy_3d_pct = usage.busy_3d_pct.min(100.0);
    usage.self_3d_pct = usage.self_3d_pct.min(100.0);
    usage.encode_pct = usage.encode_pct.min(100.0);
    usage.copy_pct = (copy_total as f32).min(100.0);
    usage
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HagsSupport {
    AlwaysOff,
    AlwaysOn,
    Experimental,
    Stable,
    Unknown,
}

impl HagsSupport {
    pub fn from_state(state: u32) -> Self {
        match state {
            0 => Self::AlwaysOff,
            1 => Self::AlwaysOn,
            2 => Self::Experimental,
            3 => Self::Stable,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AlwaysOff => "always off",
            Self::AlwaysOn => "always on",
            Self::Experimental => "experimental",
            Self::Stable => "stable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuScheduling {
    pub enabled: bool,
    pub support: HagsSupport,
}

pub fn hags_label(scheduling: Option<GpuScheduling>) -> &'static str {
    match scheduling {
        Some(GpuScheduling { enabled: true, .. }) => "on",
        Some(GpuScheduling { enabled: false, .. }) => "off",
        None => "unknown",
    }
}

pub fn support_label(scheduling: Option<GpuScheduling>) -> &'static str {
    scheduling
        .map_or(HagsSupport::Unknown, |s| s.support)
        .label()
}

pub fn mib(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}

#[cfg(test)]
mod tests;
