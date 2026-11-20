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
