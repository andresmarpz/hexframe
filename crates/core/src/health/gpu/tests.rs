use super::*;

#[test]
fn each_support_value_maps_to_its_label() {
    let labels: Vec<_> = (0..=4)
        .map(|state| HagsSupport::from_state(state).label())
        .collect();
    assert_eq!(
        labels,
        [
            "always off",
            "experimental",
            "stable",
            "always on",
            "unknown"
        ]
    );
}

#[test]
fn a_failed_query_reads_as_unknown() {
    assert_eq!(hags_label(None), "unknown");
    assert_eq!(support_label(None), "unknown");
}

#[test]
fn the_state_and_support_are_reported_together() {
    let on = Some(GpuScheduling {
        enabled: true,
        support: HagsSupport::Stable,
    });
    let off = Some(GpuScheduling {
        enabled: false,
        support: HagsSupport::Experimental,
    });
    assert_eq!((hags_label(on), support_label(on)), ("on", "stable"));
    assert_eq!(
        (hags_label(off), support_label(off)),
        ("off", "experimental")
    );
}

#[test]
fn memory_above_four_gib_prints_in_full() {
    assert_eq!(mib(8 * 1024 * 1024 * 1024), 8192);
}
