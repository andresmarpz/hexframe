use super::BoxWriter;

#[test]
fn nested_boxes_patch_their_sizes() {
    let mut w = BoxWriter::new();
    let outer = w.begin(b"moov");
    w.boxed(b"free", |w| w.bytes(&[1, 2, 3]));
    w.full_boxed(b"mvhd", 1, 0x000203, |w| w.u16(7));
    w.end(outer);
    let out = w.into_inner();

    let expected: Vec<u8> = [
        &[0, 0, 0, 8 + 11 + 14][..],
        b"moov",
        &[0, 0, 0, 11],
        b"free",
        &[1, 2, 3],
        &[0, 0, 0, 14],
        b"mvhd",
        &[1, 0, 2, 3],
        &[0, 7],
    ]
    .concat();
    assert_eq!(out, expected);
}

#[test]
fn numeric_writers_are_big_endian() {
    let mut w = BoxWriter::new();
    w.u8(1);
    w.u16(0x0203);
    w.u24(0x040506);
    w.u32(0x0708090a);
    w.u64(0x0b0c0d0e0f101112);
    w.i32(-2);
    w.fixed16_16(1.5);
    w.fixed8_8(1.0);
    assert_eq!(
        w.into_inner(),
        [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 0xff, 0xff, 0xff, 0xfe,
            0, 1, 0x80, 0, 1, 0
        ]
    );
}

#[test]
fn zeros_and_len() {
    let mut w = BoxWriter::new();
    assert!(w.is_empty());
    w.zeros(5);
    assert_eq!(w.len(), 5);
    assert_eq!(w.into_inner(), vec![0; 5]);
}
