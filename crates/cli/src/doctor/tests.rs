use super::deep::{Observation, Packet, build, count_b_pictures};
use greyscreen_core::encoder::{Nv12Means, nv12_report};

const SPS_1080P60: [u8; 21] = [
    0x67, 0x64, 0x00, 0x28, 0xAC, 0xE8, 0x07, 0x80, 0x22, 0x7E, 0x58, 0x40, 0x00, 0x00, 0x03, 0x00,
    0x40, 0x00, 0x00, 0x1E, 0x30,
];
const IDR: [u8; 7] = [0, 0, 0, 1, 0x65, 0x88, 0x00];
const P_SLICE: [u8; 7] = [0, 0, 0, 1, 0x41, 0xC0, 0x00];
const B_SLICE: [u8; 7] = [0, 0, 0, 1, 0x01, 0xA0, 0x00];
const GOOD_MEANS: Nv12Means = Nv12Means {
    luma: 125.0,
    luma_left: 30.0,
    luma_right: 220.0,
    chroma_u: 128.0,
    chroma_v: 128.0,
};

fn packet(nal: &[u8], pts_hns: i64, key: bool) -> Packet {
    Packet {
        annexb: nal.to_vec(),
        pts_hns,
        key,
    }
}

fn idr_with_headers(sps: &[u8]) -> Vec<u8> {
    let mut stream = vec![0, 0, 0, 1];
    stream.extend_from_slice(sps);
    stream.extend_from_slice(&[0, 0, 0, 1, 0x68, 0xCE, 0x3C, 0x80]);
    stream.extend_from_slice(&IDR);
    stream
}

fn clean_packets() -> Vec<Packet> {
    vec![
        packet(&idr_with_headers(&SPS_1080P60), 0, true),
        packet(&P_SLICE, 166_666, false),
        packet(&P_SLICE, 333_333, false),
    ]
}

fn observation(packets: Vec<Packet>) -> Observation {
    Observation {
        adapter: "Test GPU".to_owned(),
        vendor_id: 0x10de,
        luid: 0xabc,
        driver_version: "1.2.3.4".to_owned(),
        hardware: true,
        is_capture_adapter: true,
        encoder: "Test encoder".to_owned(),
        properties: vec!["AVEncMPVDefaultBPictureCount: set ok".to_owned()],
        cpu_readback: false,
        size: (1920, 1080),
        fps: 60,
        frames_in: 3,
        packets,
        nv12: Some(GOOD_MEANS),
        error: None,
    }
}

#[test]
fn counts_b_pictures_and_ignores_p_and_idr() {
    let mut stream = Vec::new();
    for nal in [IDR, P_SLICE, B_SLICE, P_SLICE, B_SLICE] {
        stream.extend_from_slice(&nal);
    }
    assert_eq!(count_b_pictures(&stream), 2);
    assert_eq!(count_b_pictures(&IDR), 0);
    assert_eq!(count_b_pictures(&[]), 0);
}

#[test]
fn a_continuation_slice_of_a_b_picture_is_not_counted_twice() {
    let mut stream = B_SLICE.to_vec();
    stream.extend_from_slice(&[0, 0, 1, 0x01, 0x48, 0x00]);
    assert_eq!(count_b_pictures(&stream), 1);
}

#[test]
fn clean_hardware_encode_passes() {
    let report = build(&[observation(clean_packets())], "0.1", "windows x86_64");
    assert!(report.pass);
    assert!(!report.open_bframe_bug);
    assert_eq!(report.adapters[0].b_frames, 0);
    assert!(report.adapters[0].pts_strictly_rising);
}

#[test]
fn b_frames_fail_and_flag_the_bug() {
    let mut packets = clean_packets();
    packets.push(packet(&B_SLICE, 500_000, false));
    let report = build(&[observation(packets)], "0.1", "windows x86_64");
    assert!(!report.pass);
    assert!(report.open_bframe_bug);
    assert_eq!(report.adapters[0].b_frames, 1);
}

#[test]
fn reordered_output_fails_and_flags_the_bug() {
    let mut packets = clean_packets();
    packets.swap(1, 2);
    let report = build(&[observation(packets)], "0.1", "windows x86_64");
    assert!(!report.pass);
    assert!(report.open_bframe_bug);
    assert!(!report.adapters[0].pts_strictly_rising);
}

#[test]
fn equal_timestamps_are_not_strictly_rising() {
    let mut packets = clean_packets();
    packets[2].pts_hns = packets[1].pts_hns;
    let report = build(&[observation(packets)], "0.1", "windows x86_64");
    assert!(!report.adapters[0].pass);
}

#[test]
fn a_hardware_readback_fails_but_a_software_one_does_not() {
    let mut hardware = observation(clean_packets());
    hardware.cpu_readback = true;
    let mut software = hardware.clone();
    software.hardware = false;
    let report = build(&[hardware, software], "0.1", "windows x86_64");
    assert!(!report.adapters[0].pass);
    assert!(report.adapters[1].pass);
}

#[test]
fn another_adapter_is_a_note_not_a_failure() {
    let mut other = observation(clean_packets());
    other.is_capture_adapter = false;
    let report = build(&[other], "0.1", "windows x86_64");
    assert!(report.pass);
    assert_eq!(report.adapters[0].notes.len(), 1);
}

#[test]
fn an_encode_error_and_an_empty_report_both_fail() {
    let mut broken = observation(Vec::new());
    broken.error = Some("no device".to_owned());
    assert!(!build(&[broken], "0.1", "windows").pass);
    assert!(!build(&[], "0.1", "windows").pass);
}

#[test]
fn nv12_check_flags_chroma_in_the_luma_plane_without_failing_the_run() {
    let bad = Nv12Means {
        chroma_u: 0.0,
        chroma_v: 0.0,
        ..GOOD_MEANS
    };
    assert_eq!(nv12_report(Some(GOOD_MEANS)).status, "ok");
    let flagged = nv12_report(Some(bad));
    assert_eq!(flagged.status, "suspect");
    assert!(flagged.detail.contains('U') && flagged.detail.contains('V'));
    assert_eq!(nv12_report(None).status, "unavailable");

    let mut observed = observation(clean_packets());
    observed.nv12 = Some(bad);
    assert!(build(&[observed], "0.1", "windows").pass);
}

#[test]
fn the_report_serialises_with_stable_keys() {
    let report = build(&[observation(clean_packets())], "0.1", "windows x86_64");
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["pass"], true);
    assert_eq!(json["adapters"][0]["luid"], "0000000000000abc");
    assert_eq!(json["adapters"][0]["kind"], "hardware");
    assert_eq!(json["adapters"][0]["nv12"]["status"], "ok");
    assert_eq!(json["adapters"][0]["b_frames"], 0);
}

#[test]
fn matching_headers_pass_and_are_reported() {
    let report = build(&[observation(clean_packets())], "0.1", "windows x86_64");
    let header = report.adapters[0].header.as_ref().unwrap();
    assert_eq!((header.profile, header.level), (100, 40));
    assert_eq!(header.size, "1920x1080");
    assert_eq!(header.fps, Some(60.0));
}

#[test]
fn a_30_fps_setting_against_a_60_fps_header_fails() {
    let mut slow = observation(clean_packets());
    slow.fps = 30;
    let report = build(&[slow], "0.1", "windows x86_64");
    assert!(!report.pass);
    assert!(
        report.adapters[0]
            .failures
            .iter()
            .any(|f| f.contains("fps"))
    );
}

#[test]
fn a_key_frame_without_an_sps_fails() {
    let mut bare = clean_packets();
    bare[0].annexb = IDR.to_vec();
    let report = build(&[observation(bare)], "0.1", "windows x86_64");
    assert!(!report.pass);
    assert!(report.adapters[0].header.is_none());
}

#[test]
fn a_later_key_frame_with_another_sps_fails() {
    let mut other_sps = SPS_1080P60;
    other_sps[3] = 0x29;
    let mut packets = clean_packets();
    packets.push(packet(&idr_with_headers(&other_sps), 500_000, true));
    let report = build(&[observation(packets)], "0.1", "windows x86_64");
    assert!(!report.pass);
    assert!(
        report.adapters[0]
            .failures
            .iter()
            .any(|f| f.contains("avcC"))
    );
}
