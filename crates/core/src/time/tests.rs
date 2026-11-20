use super::*;

#[test]
fn mono_now_is_monotonic() {
    let mut last = mono_now();
    for _ in 0..1000 {
        let now = mono_now();
        assert!(now >= last);
        last = now;
    }
}

#[test]
fn secs_and_hns_round_trip() {
    for secs in [0.0, 0.5, 1.0, 12.345_678, 3600.25] {
        let back = hns_to_secs(secs_to_hns(secs));
        assert!((back - secs).abs() < 1e-7, "{secs} -> {back}");
    }
    assert_eq!(secs_to_hns(1.0), HNS_PER_SEC);
}
