use crate::bytes::BoxWriter;

const SAMPLE_RATES: [u32; 13] = [
    96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
    7_350,
];
const AUDIO_OBJECT_LC: u64 = 2;
const EXPLICIT_RATE_INDEX: u64 = 15;

pub fn asc_lc(sample_rate: u32, channels: u16) -> Vec<u8> {
    let mut bits: u64 = AUDIO_OBJECT_LC;
    let mut len = 5usize;
    match SAMPLE_RATES.iter().position(|&r| r == sample_rate) {
        Some(index) => {
            bits = (bits << 4) | index as u64;
            len += 4;
        }
        None => {
            bits = (bits << 28) | (EXPLICIT_RATE_INDEX << 24) | u64::from(sample_rate & 0xFF_FFFF);
            len += 28;
        }
    }
    bits = (bits << 4) | u64::from(channels & 0xF);
    bits <<= 3;
    len += 7;
    let padded = len.div_ceil(8) * 8;
    bits <<= padded - len;
    bits.to_be_bytes()[8 - padded / 8..].to_vec()
}

fn descriptor(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut length = Vec::new();
    let mut remaining = body.len();
    loop {
        length.push((remaining & 0x7F) as u8);
        remaining >>= 7;
        if remaining == 0 {
            break;
        }
    }
    length.reverse();
    let last = length.len() - 1;
    for byte in &mut length[..last] {
        *byte |= 0x80;
    }
    let mut out = vec![tag];
    out.extend_from_slice(&length);
    out.extend_from_slice(body);
    out
}

pub fn write_esds(w: &mut BoxWriter, asc: &[u8], avg_bitrate: u32) {
    let mut decoder_config = vec![0x40, 0x15, 0, 0, 0];
    decoder_config.extend_from_slice(&avg_bitrate.to_be_bytes());
    decoder_config.extend_from_slice(&avg_bitrate.to_be_bytes());
    decoder_config.extend_from_slice(&descriptor(0x05, asc));

    let mut es = vec![0, 0, 0];
    es.extend_from_slice(&descriptor(0x04, &decoder_config));
    es.extend_from_slice(&descriptor(0x06, &[2]));

    w.full_boxed(b"esds", 0, 0, |w| w.bytes(&descriptor(0x03, &es)));
}

#[cfg(test)]
mod tests;
