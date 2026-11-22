use crate::bytes::BoxWriter;

pub const NAL_IDR: u8 = 5;
pub const NAL_SEI: u8 = 6;
pub const NAL_SPS: u8 = 7;
pub const NAL_PPS: u8 = 8;
pub const NAL_AUD: u8 = 9;
pub const NAL_PREFIX: u8 = 14;
const ASPECT_RATIO_EXTENDED: u32 = 255;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AvcInfo {
    pub idr: bool,
    pub sps: Option<Vec<u8>>,
    pub pps: Option<Vec<u8>>,
}

pub fn nal_type(header: u8) -> u8 {
    header & 0x1f
}

#[derive(Debug, Clone)]
struct NalSplitter<'a> {
    data: &'a [u8],
    pos: Option<usize>,
}

fn find_start_code(data: &[u8], from: usize) -> Option<usize> {
    data.get(from..)?
        .windows(3)
        .position(|w| w == [0, 0, 1])
        .map(|p| p + from)
}

pub fn split_annexb(data: &[u8]) -> impl Iterator<Item = &[u8]> {
    NalSplitter {
        data,
        pos: find_start_code(data, 0),
    }
}

impl<'a> Iterator for NalSplitter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        loop {
            let start = self.pos? + 3;
            let next = find_start_code(self.data, start);
            self.pos = next;
            let mut nal = &self.data[start..next.unwrap_or(self.data.len())];
            while let [rest @ .., 0] = nal {
                nal = rest;
            }
            if !nal.is_empty() {
                return Some(nal);
            }
        }
    }
}

pub fn annexb_to_avcc(data: &[u8]) -> (Vec<u8>, AvcInfo) {
    let mut out = Vec::with_capacity(data.len());
    let mut info = AvcInfo::default();
    for nal in split_annexb(data) {
        match nal_type(nal[0]) {
            NAL_SPS => info.sps = Some(nal.to_vec()),
            NAL_PPS => info.pps = Some(nal.to_vec()),
            NAL_AUD | NAL_SEI | NAL_PREFIX => {}
            kind => {
                info.idr |= kind == NAL_IDR;
                out.extend_from_slice(&(nal.len() as u32).to_be_bytes());
                out.extend_from_slice(nal);
            }
        }
    }
    (out, info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpsFields {
    pub(crate) profile: u8,
    pub(crate) level: u8,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) chroma_format: u32,
    pub(crate) bit_depth_luma: u32,
    pub(crate) bit_depth_chroma: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VuiColour {
    pub primaries: u8,
    pub transfer: u8,
    pub matrix: u8,
    pub full_range: bool,
}

pub(crate) struct BitReader {
    data: Vec<u8>,
    pos: usize,
}

impl BitReader {
    pub(crate) fn new(rbsp_with_escapes: &[u8]) -> Self {
        let mut data = Vec::with_capacity(rbsp_with_escapes.len());
        let mut zeros = 0;
        for &b in rbsp_with_escapes {
            if zeros >= 2 && b == 3 {
                zeros = 0;
                continue;
            }
            zeros = if b == 0 { zeros + 1 } else { 0 };
            data.push(b);
        }
        Self { data, pos: 0 }
    }

    pub(crate) fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.pos / 8)?;
        let bit = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Some(u32::from(bit))
    }

    pub(crate) fn bits(&mut self, count: u32) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | self.bit()?;
        }
        Some(value)
    }

    pub(crate) fn unsigned_exp(&mut self) -> Option<u32> {
        let mut zeros = 0;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        let mut value = 0u64;
        for _ in 0..zeros {
            value = (value << 1) | u64::from(self.bit()?);
        }
        u32::try_from((1u64 << zeros) - 1 + value).ok()
    }

    fn signed_exp(&mut self) -> Option<i32> {
        let k = self.unsigned_exp()?;
        let magnitude = i32::try_from(k.div_ceil(2)).ok()?;
        Some(if k % 2 == 1 { magnitude } else { -magnitude })
    }

    fn skip_scaling_list(&mut self, size: usize) -> Option<()> {
        let (mut last, mut next) = (8i32, 8i32);
        for _ in 0..size {
            if next != 0 {
                next = (last + self.signed_exp()? + 256).rem_euclid(256);
            }
            if next != 0 {
                last = next;
            }
        }
        Some(())
    }
}

fn has_chroma_fields(profile: u8) -> bool {
    matches!(
        profile,
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
    )
}

pub(crate) fn parse_sps_head(sps: &[u8]) -> Option<(SpsFields, BitReader)> {
    let (&profile, &level) = (sps.get(1)?, sps.get(3)?);
    let mut r = BitReader::new(sps.get(4..)?);
    r.unsigned_exp()?;
    let (mut chroma_format, mut bit_depth_luma, mut bit_depth_chroma) = (1, 8, 8);
    let mut separate_planes = false;
    if has_chroma_fields(profile) {
        chroma_format = r.unsigned_exp()?;
        if chroma_format == 3 {
            separate_planes = r.bit()? == 1;
        }
        bit_depth_luma = r.unsigned_exp()? + 8;
        bit_depth_chroma = r.unsigned_exp()? + 8;
        r.bit()?;
        if r.bit()? == 1 {
            for i in 0..if chroma_format != 3 { 8 } else { 12 } {
                if r.bit()? == 1 {
                    r.skip_scaling_list(if i < 6 { 16 } else { 64 })?;
                }
            }
        }
    }
    r.unsigned_exp()?;
    match r.unsigned_exp()? {
        0 => {
            r.unsigned_exp()?;
        }
        1 => {
            r.bit()?;
            r.signed_exp()?;
            r.signed_exp()?;
            for _ in 0..r.unsigned_exp()? {
                r.signed_exp()?;
            }
        }
        _ => {}
    }
    r.unsigned_exp()?;
    r.bit()?;
    let width_mbs = r.unsigned_exp()? + 1;
    let height_map_units = r.unsigned_exp()? + 1;
    let frame_mbs_only = r.bit()?;
    if frame_mbs_only == 0 {
        r.bit()?;
    }
    r.bit()?;
    let (mut left, mut right, mut top, mut bottom) = (0, 0, 0, 0);
    if r.bit()? == 1 {
        left = r.unsigned_exp()?;
        right = r.unsigned_exp()?;
        top = r.unsigned_exp()?;
        bottom = r.unsigned_exp()?;
    }
    let chroma_array_type = if separate_planes { 0 } else { chroma_format };
    let (sub_w, sub_h) = match chroma_array_type {
        1 => (2, 2),
        2 => (2, 1),
        _ => (1, 1),
    };
    let unit_x = sub_w;
    let unit_y = sub_h * (2 - frame_mbs_only);
    let width = (width_mbs * 16).checked_sub(unit_x * (left + right))?;
    let height =
        (height_map_units * 16 * (2 - frame_mbs_only)).checked_sub(unit_y * (top + bottom))?;
    let fields = SpsFields {
        profile,
        level,
        width,
        height,
        chroma_format,
        bit_depth_luma,
        bit_depth_chroma,
    };
    Some((fields, r))
}

fn parse_sps(sps: &[u8]) -> Option<SpsFields> {
    parse_sps_head(sps).map(|(fields, _)| fields)
}

fn read_vui_colour(r: &mut BitReader) -> Option<VuiColour> {
    if r.bit()? == 0 {
        return None;
    }
    if r.bit()? == 1 && r.bits(8)? == ASPECT_RATIO_EXTENDED {
        r.bits(32)?;
    }
    if r.bit()? == 1 {
        r.bit()?;
    }
    if r.bit()? == 0 {
        return None;
    }
    r.bits(3)?;
    let full_range = r.bit()? == 1;
    if r.bit()? == 0 {
        return None;
    }
    Some(VuiColour {
        primaries: r.bits(8)? as u8,
        transfer: r.bits(8)? as u8,
        matrix: r.bits(8)? as u8,
        full_range,
    })
}

pub fn parse_sps_colour(sps: &[u8]) -> Option<VuiColour> {
    let (_, mut reader) = parse_sps_head(sps)?;
    read_vui_colour(&mut reader)
}

pub fn parse_sps_dims(sps: &[u8]) -> Option<(u32, u32, u8, u8)> {
    parse_sps(sps).map(|f| (f.width, f.height, f.profile, f.level))
}

pub(crate) fn write_avcc_box(w: &mut BoxWriter, sps: &[u8], pps: &[u8]) {
    w.boxed(b"avcC", |w| write_avcc_record(w, sps, pps));
}

pub fn avcc_record(sps: &[u8], pps: &[u8]) -> Vec<u8> {
    let mut w = BoxWriter::new();
    write_avcc_record(&mut w, sps, pps);
    w.into_inner()
}

fn write_avcc_record(w: &mut BoxWriter, sps: &[u8], pps: &[u8]) {
    let byte = |i: usize| sps.get(i).copied().unwrap_or(0);
    let profile = byte(1);
    w.u8(1);
    w.u8(profile);
    w.u8(byte(2));
    w.u8(byte(3));
    w.u8(0xFF);
    w.u8(0xE1);
    w.u16(sps.len() as u16);
    w.bytes(sps);
    w.u8(1);
    w.u16(pps.len() as u16);
    w.bytes(pps);
    if matches!(profile, 100 | 110 | 122 | 144 | 244) {
        let fields = parse_sps(sps);
        let chroma = fields.map_or(1, |f| f.chroma_format);
        let luma = fields.map_or(8, |f| f.bit_depth_luma);
        let chroma_depth = fields.map_or(8, |f| f.bit_depth_chroma);
        w.u8(0xFC | (chroma & 3) as u8);
        w.u8(0xF8 | (luma.saturating_sub(8) & 7) as u8);
        w.u8(0xF8 | (chroma_depth.saturating_sub(8) & 7) as u8);
        w.u8(0);
    }
}

#[cfg(test)]
mod tests;
