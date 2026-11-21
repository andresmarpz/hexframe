use super::pump::to_annexb;

const IDR: [u8; 3] = [0x65, 0x88, 0x84];
const SLICE: [u8; 3] = [0x41, 0x9a, 0x01];
const START: [u8; 4] = [0, 0, 0, 1];

fn expected(nals: &[&[u8]]) -> Vec<u8> {
    nals.iter().flat_map(|n| [&START[..], n].concat()).collect()
}

#[test]
fn annexb_input_is_normalised_to_four_byte_start_codes() {
    let raw = [&[0, 0, 1][..], &IDR, &START, &SLICE].concat();
    assert_eq!(to_annexb(&raw).unwrap(), expected(&[&IDR, &SLICE]));
}

#[test]
fn length_prefixed_input_gets_start_codes() {
    let raw = [&[0, 0, 0, 3][..], &IDR, &[0, 0, 0, 3], &SLICE].concat();
    assert_eq!(to_annexb(&raw).unwrap(), expected(&[&IDR, &SLICE]));
}

#[test]
fn prefix_nal_units_are_dropped() {
    let raw = [&START[..], &[0x6e, 0x01], &START, &SLICE].concat();
    assert_eq!(to_annexb(&raw).unwrap(), expected(&[&SLICE]));
}

#[test]
fn garbage_is_rejected() {
    assert!(to_annexb(&[9, 9, 9, 9, 9]).is_err());
    assert!(to_annexb(&[]).is_err());
}
