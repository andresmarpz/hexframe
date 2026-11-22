use crate::avc::{self, BitReader, NAL_IDR, NAL_SPS};

const ASPECT_RATIO_EXTENDED: u32 = 255;
const SAMPLE_ASPECT_RATIOS: [(u32, u32); 17] = [
    (0, 0),
    (1, 1),
    (12, 11),
    (10, 11),
    (16, 11),
    (40, 33),
    (24, 11),
    (20, 11),
    (32, 11),
    (80, 33),
    (18, 11),
    (15, 11),
    (64, 33),
    (160, 99),
    (4, 3),
    (3, 2),
    (2, 1),
];
const ASPECT_TOLERANCE: f64 = 0.01;
const FPS_TOLERANCE: f64 = 0.05;
const PROFILE_HIGH: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VuiTiming {
    pub num_units_in_tick: u32,
    pub time_scale: u32,
    pub fixed_frame_rate: bool,
}

impl VuiTiming {
    pub fn fps(&self) -> Option<f64> {
        (self.num_units_in_tick > 0)
            .then(|| f64::from(self.time_scale) / (2.0 * f64::from(self.num_units_in_tick)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpsInfo {
    pub profile: u8,
    pub level: u8,
    pub width: u32,
    pub height: u32,
    pub sample_aspect: Option<(u32, u32)>,
    pub timing: Option<VuiTiming>,
}

impl SpsInfo {
    pub fn display_aspect(&self) -> f64 {
        let (sar_w, sar_h) = match self.sample_aspect {
            Some((w, h)) if w > 0 && h > 0 => (w, h),
            _ => (1, 1),
        };
        f64::from(self.width) * f64::from(sar_w) / (f64::from(self.height) * f64::from(sar_h))
    }

    pub fn fps(&self) -> Option<f64> {
        self.timing.and_then(|t| t.fps())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expected {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeaderCheck {
    pub failures: Vec<String>,
    pub notes: Vec<String>,
}

pub fn parse_sps_info(sps: &[u8]) -> Option<SpsInfo> {
    let (fields, mut reader) = avc::parse_sps_head(sps)?;
    let (sample_aspect, timing) = read_vui(&mut reader).unwrap_or((None, None));
    Some(SpsInfo {
        profile: fields.profile,
        level: fields.level,
        width: fields.width,
        height: fields.height,
        sample_aspect,
        timing,
    })
}

type Vui = (Option<(u32, u32)>, Option<VuiTiming>);

fn read_vui(r: &mut BitReader) -> Option<Vui> {
    if r.bit()? == 0 {
        return Some((None, None));
    }
    let mut aspect = None;
    if r.bit()? == 1 {
        let idc = r.bits(8)?;
        aspect = if idc == ASPECT_RATIO_EXTENDED {
            Some((r.bits(16)?, r.bits(16)?))
        } else {
            SAMPLE_ASPECT_RATIOS.get(idc as usize).copied()
        };
    }
    if r.bit()? == 1 {
        r.bit()?;
    }
    if r.bit()? == 1 {
        r.bits(4)?;
        if r.bit()? == 1 {
            r.bits(24)?;
        }
    }
    if r.bit()? == 1 {
        r.unsigned_exp()?;
        r.unsigned_exp()?;
    }
    let timing = if r.bit()? == 1 {
        Some(VuiTiming {
            num_units_in_tick: r.bits(32)?,
            time_scale: r.bits(32)?,
            fixed_frame_rate: r.bit()? == 1,
        })
    } else {
        None
    };
    Some((aspect, timing))
}

pub fn check_sps(sps: &[u8], expected: Expected) -> HeaderCheck {
    let mut check = HeaderCheck::default();
    let Some(info) = parse_sps_info(sps) else {
        check
            .failures
            .push("the SPS could not be parsed".to_owned());
        return check;
    };
    if (info.width, info.height) != (expected.width, expected.height) {
        check.failures.push(format!(
            "the SPS says {}x{} but the setting is {}x{}",
            info.width, info.height, expected.width, expected.height
        ));
    }
    let wanted_aspect = f64::from(expected.width) / f64::from(expected.height.max(1));
    if (info.display_aspect() / wanted_aspect - 1.0).abs() > ASPECT_TOLERANCE {
        check.failures.push(format!(
            "the SPS display aspect is {:.3} but the setting is {wanted_aspect:.3}",
            info.display_aspect()
        ));
    }
    match info.fps() {
        Some(fps) if (fps / f64::from(expected.fps.max(1)) - 1.0).abs() > FPS_TOLERANCE => {
            check.failures.push(format!(
                "the SPS timing says {fps:.2} fps but the setting is {} fps",
                expected.fps
            ));
        }
        Some(_) => {}
        None => check.notes.push("the SPS has no VUI timing".to_owned()),
    }
    if info.profile != PROFILE_HIGH {
        check.notes.push(format!(
            "the SPS profile is {} instead of High ({PROFILE_HIGH})",
            info.profile
        ));
    }
    check
}

pub fn sps_from_avcc(record: &[u8]) -> Option<&[u8]> {
    if record.first() != Some(&1) || record.get(5)? & 0x1f == 0 {
        return None;
    }
    let len = usize::from(u16::from_be_bytes([*record.get(6)?, *record.get(7)?]));
    record.get(8..8 + len)
}

pub fn first_idr_sps(annexb: &[u8]) -> Option<&[u8]> {
    let mut sps = None;
    for nal in avc::split_annexb(annexb) {
        match avc::nal_type(nal[0]) {
            NAL_SPS => sps = Some(nal),
            NAL_IDR => return sps,
            _ => {}
        }
    }
    None
}

pub fn check_avcc_matches_idr(avcc_record: &[u8], idr_annexb: &[u8]) -> Option<String> {
    let Some(avcc_sps) = sps_from_avcc(avcc_record) else {
        return Some("the avcC holds no SPS".to_owned());
    };
    let Some(idr_sps) = first_idr_sps(idr_annexb) else {
        return Some("the key frame carries no SPS".to_owned());
    };
    (avcc_sps != idr_sps).then(|| "the avcC SPS differs from the key frame SPS".to_owned())
}

#[cfg(test)]
mod tests;
