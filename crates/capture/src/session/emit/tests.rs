use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::media::{MediaEvent, Track};
use greyscreen_mp4::avc::{nal_type, parse_sps_dims};

use super::{Sink, StartInfo};
use crate::h264::EncodedVideo;

const SPS: [u8; 21] = [
    0x67, 0x42, 0xC0, 0x1E, 0xDA, 0x05, 0x07, 0xE8, 0x40, 0x00, 0x00, 0x03, 0x00, 0x40, 0x00, 0x00,
    0x0C, 0x23, 0xC5, 0x8B, 0xA8,
];
const PPS: [u8; 2] = [0x68, 0xCE];
const IDR: [u8; 3] = [0x65, 0x88, 0x84];
const START: [u8; 4] = [0, 0, 0, 1];

fn sink() -> (Sink, Receiver<MediaEvent>) {
    let (tx, rx) = unbounded();
    let info = StartInfo {
        t0: 0,
        width: 640,
        height: 360,
        fps: 60,
        audio: None,
        encoder: "fake".into(),
    };
    (Sink::new(tx, info), rx)
}

fn annexb(nals: &[&[u8]]) -> Vec<u8> {
    nals.iter()
        .flat_map(|nal| [&START[..], nal].concat())
        .collect()
}

fn packet(nals: &[&[u8]]) -> Vec<EncodedVideo> {
    vec![EncodedVideo {
        annexb: annexb(nals),
        pts_hns: 0,
        key: true,
    }]
}

fn started_format(events: &Receiver<MediaEvent>) -> (Vec<u8>, Vec<u8>) {
    let all: Vec<MediaEvent> = events.try_iter().collect();
    let Some(MediaEvent::Started { video, .. }) = all.first() else {
        panic!("the first event must be Started: {all:?}");
    };
    let keys = all
        .iter()
        .filter(|event| matches!(event, MediaEvent::Packet(p) if p.track == Track::Video && p.key))
        .count();
    assert_eq!(keys, 1, "{all:?}");
    (video.sps.clone(), video.pps.clone())
}

#[test]
fn an_encoder_with_its_header_only_in_the_first_packet_gives_a_valid_start() {
    let (mut sink, events) = sink();
    sink.video(&mut packet(&[&SPS, &PPS, &IDR]));
    let (sps, pps) = started_format(&events);
    assert_eq!(nal_type(sps[0]), 7);
    assert_eq!(nal_type(pps[0]), 8);
    assert_eq!(sps, SPS);
    assert_eq!(pps, PPS);
    let (width, height, profile, level) = parse_sps_dims(&sps).unwrap();
    assert_eq!((profile, level), (66, 30));
    assert!(width > 0 && height > 0);
    assert_eq!(sink.take_fault(), None);
}

#[test]
fn an_encoder_with_its_header_only_in_the_output_type_starts_from_the_fallback() {
    let (mut sink, events) = sink();
    sink.set_fallback(SPS.to_vec(), PPS.to_vec());
    sink.video(&mut packet(&[&IDR]));
    let (sps, pps) = started_format(&events);
    assert_eq!((sps, pps), (SPS.to_vec(), PPS.to_vec()));
    assert_eq!(sink.take_fault(), None);
}
