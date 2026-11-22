use super::*;
use crate::reader::boxes;

const FIXTURE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/v.h264"
));

fn first_nal(kind: u8) -> Vec<u8> {
    split_annexb(FIXTURE)
        .find(|n| nal_type(n[0]) == kind)
        .map(<[u8]>::to_vec)
        .unwrap_or_default()
}

#[test]
fn split_handles_three_and_four_byte_start_codes_and_trailing_zeros() {
    let stream = [
        0, 0, 0, 1, 0x67, 1, 2, 0, 0, 0, 0, 0, 1, 0x68, 3, 0, 0, 1, 0x65, 4, 5, 0, 0,
    ];
    let nals: Vec<&[u8]> = split_annexb(&stream).collect();
    assert_eq!(nals, [&[0x67, 1, 2][..], &[0x68, 3], &[0x65, 4, 5]]);
}

#[test]
fn split_skips_empty_nals_and_garbage_before_first_start_code() {
    let stream = [9, 9, 0, 0, 1, 0, 0, 1, 0x41, 7];
    let nals: Vec<&[u8]> = split_annexb(&stream).collect();
    assert_eq!(nals, [&[0x41, 7][..]]);
    assert_eq!(split_annexb(&[1, 2, 3]).count(), 0);
}

#[test]
fn nal_type_masks_the_header() {
    assert_eq!(nal_type(0x67), NAL_SPS);
    assert_eq!(nal_type(0x65), NAL_IDR);
    assert_eq!(nal_type(0x41), 1);
}

#[test]
fn annexb_to_avcc_drops_parameter_sets_and_reports_them() {
    let stream = [
        0, 0, 0, 1, 0x09, 0xF0, 0, 0, 0, 1, 0x67, 1, 2, 3, 0, 0, 0, 1, 0x68, 4, 0, 0, 1, 0x06, 9,
        9, 0, 0, 1, 0x65, 0xAA, 0xBB, 0xCC, 0, 0, 1, 0x65, 0xDD,
    ];
    let (avcc, info) = annexb_to_avcc(&stream);
    assert_eq!(
        avcc,
        [0, 0, 0, 4, 0x65, 0xAA, 0xBB, 0xCC, 0, 0, 0, 2, 0x65, 0xDD]
    );
    assert!(info.idr);
    assert_eq!(info.sps.as_deref(), Some(&[0x67, 1, 2, 3][..]));
    assert_eq!(info.pps.as_deref(), Some(&[0x68, 4][..]));
}

#[test]
fn non_idr_access_unit_is_not_marked_idr() {
    let (avcc, info) = annexb_to_avcc(&[0, 0, 0, 1, 0x41, 1, 2]);
    assert_eq!(avcc, [0, 0, 0, 3, 0x41, 1, 2]);
    assert!(!info.idr);
    assert_eq!(info.sps, None);
}

#[test]
fn round_trip_keeps_every_slice_of_the_fixture() {
    let (avcc, info) = annexb_to_avcc(FIXTURE);
    assert!(info.idr);

    let mut from_avcc = Vec::new();
    let mut pos = 0;
    while pos < avcc.len() {
        let len = u32::from_be_bytes([avcc[pos], avcc[pos + 1], avcc[pos + 2], avcc[pos + 3]]);
        let end = pos + 4 + len as usize;
        from_avcc.push(&avcc[pos + 4..end]);
        pos = end;
    }
    assert_eq!(pos, avcc.len());

    let slices: Vec<&[u8]> = split_annexb(FIXTURE)
        .filter(|n| matches!(nal_type(n[0]), 1 | NAL_IDR))
        .collect();
    assert_eq!(from_avcc, slices);
    assert!(slices.len() >= 120);
}

#[test]
fn sps_from_real_x264_stream_parses_with_cropping() {
    let sps = first_nal(NAL_SPS);
    assert_eq!(parse_sps_dims(&sps), Some((640, 360, 100, sps[3])));
}

#[test]
fn truncated_sps_does_not_parse() {
    let sps = first_nal(NAL_SPS);
    assert_eq!(parse_sps_dims(&sps[..6]), None);
    assert_eq!(parse_sps_dims(&[0x67]), None);
}

#[test]
fn baseline_sps_without_chroma_fields_parses() {
    let sps = [
        0x67, 0x42, 0xC0, 0x1E, 0xDA, 0x05, 0x07, 0xE8, 0x40, 0x00, 0x00, 0x03, 0x00, 0x40, 0x00,
        0x00, 0x0C, 0x23, 0xC5, 0x8B, 0xA8,
    ];
    let (_, _, profile, level) = parse_sps_dims(&sps).unwrap_or_default();
    assert_eq!((profile, level), (66, 30));
}

#[test]
fn avcc_box_carries_sps_pps_and_high_profile_extension() {
    let sps = first_nal(NAL_SPS);
    let pps = first_nal(NAL_PPS);
    let mut w = BoxWriter::new();
    write_avcc_box(&mut w, &sps, &pps);
    let out = w.into_inner();

    let top = boxes(&out).next().unwrap_or_else(|| panic!("no box"));
    assert_eq!(&top.fourcc, b"avcC");
    let body = top.payload_bytes(&out);
    assert_eq!(&body[..6], &[1, 100, sps[2], sps[3], 0xFF, 0xE1]);
    let sps_len = u16::from_be_bytes([body[6], body[7]]) as usize;
    assert_eq!(&body[8..8 + sps_len], &sps[..]);
    let after_sps = 8 + sps_len;
    assert_eq!(body[after_sps], 1);
    let pps_len = u16::from_be_bytes([body[after_sps + 1], body[after_sps + 2]]) as usize;
    assert_eq!(&body[after_sps + 3..after_sps + 3 + pps_len], &pps[..]);
    assert_eq!(&body[after_sps + 3 + pps_len..], &[0xFD, 0xF8, 0xF8, 0]);
}

#[test]
fn avcc_box_for_baseline_has_no_extension() {
    let mut w = BoxWriter::new();
    write_avcc_box(&mut w, &[0x67, 0x42, 0xC0, 0x1E, 0xAA], &[0x68, 0xCE]);
    assert_eq!(w.len(), 8 + 6 + 2 + 5 + 1 + 2 + 2);
}

fn baseline_sps_with_vui(vui: &[bool]) -> Vec<u8> {
    let mut bits = vec![
        true, true, false, true, true, false, true, false, false, true, true, true, true, false,
    ];
    bits.extend_from_slice(vui);
    bits.push(true);
    while bits.len() % 8 != 0 {
        bits.push(false);
    }
    let mut sps = vec![0x67, 66, 0xC0, 30];
    sps.extend(
        bits.chunks(8)
            .map(|byte| byte.iter().fold(0, |a, &b| a << 1 | u8::from(b))),
    );
    sps
}

fn byte_bits(value: u8) -> impl Iterator<Item = bool> {
    (0..8).rev().map(move |i| value >> i & 1 == 1)
}

#[test]
fn vui_colour_is_read_from_the_sps() {
    let mut vui = vec![true, false, false, true, true, false, true, false, true];
    vui.extend(byte_bits(1).chain(byte_bits(1)).chain(byte_bits(1)));
    assert_eq!(
        parse_sps_colour(&baseline_sps_with_vui(&vui)),
        Some(VuiColour {
            primaries: 1,
            transfer: 1,
            matrix: 1,
            full_range: false
        })
    );
    vui[7] = true;
    assert_eq!(
        parse_sps_colour(&baseline_sps_with_vui(&vui)).map(|c| c.full_range),
        Some(true)
    );
}

#[test]
fn sps_without_colour_description_has_none() {
    assert_eq!(parse_sps_colour(&baseline_sps_with_vui(&[false])), None);
    let no_description = [true, false, false, true, false, false, false, false, false];
    assert_eq!(
        parse_sps_colour(&baseline_sps_with_vui(&no_description)),
        None
    );
    assert_eq!(parse_sps_colour(&first_nal(NAL_SPS)), None);
}
