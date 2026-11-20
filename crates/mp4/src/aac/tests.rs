use super::*;

#[test]
fn asc_for_48khz_stereo_is_0x11_0x90() {
    assert_eq!(asc_lc(48_000, 2), [0x11, 0x90]);
}

#[test]
fn asc_for_44khz_stereo_and_48khz_mono() {
    assert_eq!(asc_lc(44_100, 2), [0x12, 0x10]);
    assert_eq!(asc_lc(48_000, 1), [0x11, 0x88]);
}

#[test]
fn asc_for_unlisted_rate_uses_explicit_frequency() {
    let asc = asc_lc(50_000, 2);
    assert_eq!(asc.len(), 5);
    assert_eq!(asc[0], 0x17);
    let bits = u64::from_be_bytes([0, 0, 0, asc[0], asc[1], asc[2], asc[3], asc[4]]);
    assert_eq!((bits >> 7) & 0xFF_FFFF, 50_000);
    assert_eq!((bits >> 3) & 0xF, 2);
}

#[test]
fn esds_nests_the_descriptors_around_the_asc() {
    let mut w = BoxWriter::new();
    write_esds(&mut w, &[0x11, 0x90], 192_000);
    let bitrate = 192_000u32.to_be_bytes();
    let expected: Vec<u8> = [
        &[0, 0, 0, 39][..],
        b"esds",
        &[0, 0, 0, 0],
        &[0x03, 0x19, 0, 0, 0],
        &[0x04, 0x11, 0x40, 0x15, 0, 0, 0],
        &bitrate,
        &bitrate,
        &[0x05, 0x02, 0x11, 0x90],
        &[0x06, 0x01, 0x02],
    ]
    .concat();
    assert_eq!(w.into_inner(), expected);
}

#[test]
fn long_descriptor_lengths_use_the_expandable_form() {
    let body = vec![7u8; 200];
    let out = descriptor(0x05, &body);
    assert_eq!(&out[..3], &[0x05, 0x81, 0x48]);
    assert_eq!(out.len(), 3 + 200);
}

fn adts_frame(payload: &[u8], protected: bool) -> Vec<u8> {
    let header_len = if protected { 9 } else { 7 };
    let len = header_len + payload.len();
    let mut out = vec![
        0xFF,
        if protected { 0xF0 } else { 0xF1 },
        0x4C,
        0x80 | (len >> 11) as u8 & 3,
        (len >> 3) as u8,
        ((len & 7) << 5) as u8 | 0x1F,
        0xFC,
    ];
    out.resize(header_len, 0);
    out.extend_from_slice(payload);
    out
}

#[test]
fn split_adts_strips_headers_with_and_without_crc() {
    let mut stream = adts_frame(&[1, 2, 3], false);
    stream.extend(adts_frame(&[4, 5], true));
    stream.extend(adts_frame(&[6], false));
    assert_eq!(split_adts(&stream), [&[1, 2, 3][..], &[4, 5], &[6]]);
}

#[test]
fn split_adts_stops_at_truncated_or_invalid_data() {
    let mut stream = adts_frame(&[1, 2, 3], false);
    let mut cut = adts_frame(&[9; 10], false);
    cut.truncate(12);
    stream.extend(cut);
    assert_eq!(split_adts(&stream), [&[1, 2, 3][..]]);
    assert!(split_adts(&[0, 1, 2, 3, 4, 5, 6, 7]).is_empty());
    assert!(split_adts(&[]).is_empty());
}
