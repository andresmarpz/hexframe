use super::*;

struct Bits {
    bytes: Vec<u8>,
    used: usize,
}

impl Bits {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            used: 0,
        }
    }

    fn put(&mut self, value: u64, count: u32) {
        for i in (0..count).rev() {
            if self.used % 8 == 0 {
                self.bytes.push(0);
            }
            let bit = ((value >> i) & 1) as u8;
            let last = self.bytes.len() - 1;
            self.bytes[last] |= bit << (7 - self.used % 8);
            self.used += 1;
        }
    }

    fn ue(&mut self, value: u32) {
        let v = u64::from(value) + 1;
        let len = 64 - v.leading_zeros();
        self.put(0, len - 1);
        self.put(v, len);
    }
}

struct VuiSpec {
    aspect: Option<(u32, u32)>,
    timing: Option<(u32, u32)>,
}

fn sps(profile: u8, width: u32, height: u32, crop_bottom: u32, vui: Option<VuiSpec>) -> Vec<u8> {
    let mut b = Bits::new();
    b.ue(0);
    if matches!(profile, 100 | 110 | 122 | 244) {
        b.ue(1);
        b.ue(0);
        b.ue(0);
        b.put(0, 1);
        b.put(0, 1);
    }
    b.ue(0);
    b.ue(0);
    b.ue(0);
    b.ue(1);
    b.put(0, 1);
    b.ue(width.div_ceil(16) - 1);
    b.ue(height.div_ceil(16) - 1);
    b.put(1, 1);
    b.put(1, 1);
    if crop_bottom > 0 {
        b.put(1, 1);
        b.ue(0);
        b.ue(0);
        b.ue(0);
        b.ue(crop_bottom);
    } else {
        b.put(0, 1);
    }
    match vui {
        None => b.put(0, 1),
        Some(vui) => {
            b.put(1, 1);
            match vui.aspect {
                None => b.put(0, 1),
                Some((w, h)) => {
                    b.put(1, 1);
                    b.put(255, 8);
                    b.put(u64::from(w), 16);
                    b.put(u64::from(h), 16);
                }
            }
            b.put(0, 1);
            b.put(0, 1);
            b.put(0, 1);
            match vui.timing {
                None => b.put(0, 1),
                Some((units, scale)) => {
                    b.put(1, 1);
                    b.put(u64::from(units), 32);
                    b.put(u64::from(scale), 32);
                    b.put(1, 1);
                }
            }
        }
    }
    b.put(1, 1);
    let mut out = vec![0x67, profile, 0, 40];
    out.extend(b.bytes);
    out
}

fn high_1080p60() -> Vec<u8> {
    sps(
        100,
        1920,
        1080,
        4,
        Some(VuiSpec {
            aspect: Some((1, 1)),
            timing: Some((1, 120)),
        }),
    )
}

const WANT: Expected = Expected {
    width: 1920,
    height: 1080,
    fps: 60,
};

#[test]
fn parses_high_profile_1080p_with_cropping_and_a_60_fps_vui() {
    let info = parse_sps_info(&high_1080p60()).unwrap();
    assert_eq!((info.profile, info.level), (100, 40));
    assert_eq!((info.width, info.height), (1920, 1080));
    assert_eq!(info.sample_aspect, Some((1, 1)));
    assert_eq!(info.fps(), Some(60.0));
    assert!(info.timing.unwrap().fixed_frame_rate);
}

#[test]
fn parses_the_real_fixture_sps() {
    let fixture = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/v.h264"
    ));
    let sps = avc::split_annexb(fixture)
        .find(|n| avc::nal_type(n[0]) == NAL_SPS)
        .unwrap();
    let info = parse_sps_info(sps).unwrap();
    assert!(info.width > 0 && info.height > 0);
}

#[test]
fn a_sps_without_vui_parses_with_no_timing() {
    let info = parse_sps_info(&sps(100, 1280, 720, 0, None)).unwrap();
    assert_eq!((info.width, info.height), (1280, 720));
    assert_eq!(info.timing, None);
    assert_eq!(info.sample_aspect, None);
}

#[test]
fn matching_headers_pass() {
    assert_eq!(check_sps(&high_1080p60(), WANT), HeaderCheck::default());
}

#[test]
fn flags_a_30_fps_vui_on_a_60_fps_setting() {
    let slow = sps(
        100,
        1920,
        1080,
        4,
        Some(VuiSpec {
            aspect: None,
            timing: Some((1, 60)),
        }),
    );
    let check = check_sps(&slow, WANT);
    assert_eq!(check.failures.len(), 1);
    assert!(check.failures[0].contains("30.00 fps"));
}

#[test]
fn flags_a_wrong_display_aspect() {
    let wide = sps(
        100,
        1920,
        1080,
        4,
        Some(VuiSpec {
            aspect: Some((4, 3)),
            timing: Some((1, 120)),
        }),
    );
    let check = check_sps(&wide, WANT);
    assert_eq!(check.failures.len(), 1);
    assert!(check.failures[0].contains("aspect"));
}

#[test]
fn flags_a_wrong_size_and_notes_another_profile() {
    let check = check_sps(&sps(77, 1280, 720, 0, None), WANT);
    assert!(check.failures.iter().any(|f| f.contains("1280x720")));
    assert!(check.notes.iter().any(|n| n.contains("profile is 77")));
}

#[test]
fn garbage_is_a_failure_not_a_panic() {
    assert_eq!(check_sps(&[0x67], WANT).failures.len(), 1);
}

fn annexb_idr(sps: &[u8]) -> Vec<u8> {
    let mut out = vec![0, 0, 0, 1];
    out.extend_from_slice(sps);
    out.extend_from_slice(&[
        0, 0, 0, 1, 0x68, 0xCE, 0x3C, 0x80, 0, 0, 0, 1, 0x65, 0x88, 0x84,
    ]);
    out
}

#[test]
fn avcc_matching_the_idr_sps_passes() {
    let sps = high_1080p60();
    let record = avc::avcc_record(&sps, &[0x68, 0xCE]);
    assert_eq!(sps_from_avcc(&record), Some(&sps[..]));
    assert_eq!(check_avcc_matches_idr(&record, &annexb_idr(&sps)), None);
}

#[test]
fn avcc_that_differs_from_the_idr_sps_is_flagged() {
    let record = avc::avcc_record(&high_1080p60(), &[0x68, 0xCE]);
    let other = sps(100, 1920, 1080, 4, None);
    let message = check_avcc_matches_idr(&record, &annexb_idr(&other)).unwrap();
    assert!(message.contains("differs"));
}

#[test]
fn missing_sps_is_reported() {
    let record = avc::avcc_record(&high_1080p60(), &[0x68]);
    assert!(check_avcc_matches_idr(&record, &[0, 0, 0, 1, 0x65, 1]).is_some());
    assert!(check_avcc_matches_idr(&[1, 100, 0, 40, 0xFF, 0xE0], &[]).is_some());
}
