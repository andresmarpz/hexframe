use serde::{Deserialize, Serialize};

pub(crate) const RESOLUTION_LADDER: [u32; 6] = [2160, 1440, 1200, 1080, 900, 720];
pub(crate) const MIN_HEIGHT: u32 = 144;
pub(crate) const MAX_HEIGHT: u32 = 4320;
const DEFAULT_HEIGHT: u32 = 1080;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ResolutionRepr")]
pub enum Resolution {
    Native,
    Height(u32),
}

#[derive(Deserialize)]
enum ResolutionRepr {
    Native,
    Height(u32),
    P1440,
    P1080,
    P720,
}

impl From<ResolutionRepr> for Resolution {
    fn from(repr: ResolutionRepr) -> Self {
        match repr {
            ResolutionRepr::Native => Resolution::Native,
            ResolutionRepr::Height(h) => Resolution::Height(h),
            ResolutionRepr::P1440 => Resolution::Height(1440),
            ResolutionRepr::P1080 => Resolution::Height(1080),
            ResolutionRepr::P720 => Resolution::Height(720),
        }
    }
}

impl Default for Resolution {
    fn default() -> Self {
        Resolution::Height(DEFAULT_HEIGHT)
    }
}

impl Resolution {
    pub fn height(self) -> Option<u32> {
        match self {
            Resolution::Native => None,
            Resolution::Height(h) => Some(h),
        }
    }

    pub(super) fn clamped(self) -> Self {
        match self {
            Resolution::Native => Resolution::Native,
            Resolution::Height(h) => Resolution::Height(h.clamp(MIN_HEIGHT, MAX_HEIGHT)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionOption {
    pub resolution: Resolution,
    pub label: String,
}

pub fn resolution_options(displays: &[(u32, u32)], current: Resolution) -> Vec<ResolutionOption> {
    let tallest = displays
        .iter()
        .copied()
        .filter(|&(w, h)| w > 0 && h > 0)
        .max_by_key(|&(_, h)| h);
    let label = |height: u32| match tallest {
        Some((w, h)) => format!("{} × {height}", scaled_width(w, h, height)),
        None => format!("{height}p"),
    };
    let native_label = match tallest {
        Some((w, h)) => format!("Native ({w} × {h})"),
        None => "Native".to_owned(),
    };
    let mut heights: Vec<u32> = RESOLUTION_LADDER
        .into_iter()
        .filter(|&h| tallest.is_none_or(|(_, native)| h < native))
        .collect();
    if let Resolution::Height(saved) = current
        && !heights.contains(&saved)
    {
        heights.push(saved);
        heights.sort_unstable_by(|a, b| b.cmp(a));
    }
    let mut options = vec![ResolutionOption {
        resolution: Resolution::Native,
        label: native_label,
    }];
    options.extend(heights.into_iter().map(|h| ResolutionOption {
        resolution: Resolution::Height(h),
        label: label(h),
    }));
    options
}

fn scaled_width(width: u32, height: u32, target: u32) -> u32 {
    let exact = f64::from(width) * f64::from(target) / f64::from(height);
    ((exact / 2.0).round() as u32) * 2
}
