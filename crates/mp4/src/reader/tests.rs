use super::*;
use crate::bytes::BoxWriter;

fn sample_box(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut w = BoxWriter::new();
    w.boxed(fourcc, |w| w.bytes(payload));
    w.into_inner()
}

#[test]
fn iterates_siblings_and_children() {
    let inner = [sample_box(b"aaaa", &[1, 2]), sample_box(b"bbbb", &[3])].concat();
    let data = [sample_box(b"moov", &inner), sample_box(b"free", &[])].concat();

    let top: Vec<BoxRef> = boxes(&data).collect();
    assert_eq!(top.len(), 2);
    assert_eq!(&top[0].fourcc, b"moov");
    assert_eq!(top[0].size, 8 + inner.len() as u64);
    assert_eq!(top[0].payload_bytes(&data), &inner[..]);
    assert!(top.iter().all(|b| b.complete));
    assert_eq!(top[1].start, top[0].end());
    assert!(top[1].payload.is_empty());

    let kids: Vec<BoxRef> = top[0].children(&data).collect();
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].payload_bytes(&data), &[1, 2]);
    assert_eq!(
        top[0].find(&data, b"bbbb").map(|b| b.payload_bytes(&data)),
        Some(&[3u8][..])
    );
    assert_eq!(
        find_box(&data, b"free").map(|b| b.start),
        Some(top[1].start)
    );
    assert_eq!(find_box(&data, b"nope"), None);
}

#[test]
fn reads_64_bit_largesize() {
    let mut data = Vec::new();
    data.extend_from_slice(&1u32.to_be_bytes());
    data.extend_from_slice(b"mdat");
    data.extend_from_slice(&(16u64 + 3).to_be_bytes());
    data.extend_from_slice(&[7, 8, 9]);

    let b = boxes(&data).next().unwrap_or_else(|| panic!("no box"));
    assert_eq!(&b.fourcc, b"mdat");
    assert_eq!(b.size, 19);
    assert_eq!(b.payload, 16..19);
    assert!(b.complete);
}

#[test]
fn size_zero_runs_to_the_end() {
    let mut data = sample_box(b"ftyp", &[1]);
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(b"mdat");
    data.extend_from_slice(&[1, 2, 3, 4]);

    let all: Vec<BoxRef> = boxes(&data).collect();
    assert_eq!(all.len(), 2);
    assert_eq!(all[1].payload_bytes(&data), &[1, 2, 3, 4]);
    assert!(all[1].complete);
}

#[test]
fn truncated_box_is_flagged_and_stops_iteration() {
    let mut data = sample_box(b"ftyp", &[1]);
    let mut cut = sample_box(b"mdat", &[0; 20]);
    cut.truncate(15);
    data.extend_from_slice(&cut);

    let all: Vec<BoxRef> = boxes(&data).collect();
    assert_eq!(all.len(), 2);
    assert!(all[0].complete);
    assert!(!all[1].complete);
    assert_eq!(all[1].size, 28);
    assert_eq!(all[1].payload.len(), 7);
}

#[test]
fn invalid_size_and_short_tail_end_iteration() {
    let mut data = sample_box(b"ftyp", &[]);
    data.extend_from_slice(&4u32.to_be_bytes());
    data.extend_from_slice(b"bad!");
    assert_eq!(boxes(&data).count(), 1);
    assert_eq!(boxes(&[0, 0, 0]).count(), 0);
}
