use super::live_report;

#[test]
fn counts_only_live_object_lines() {
    let report = live_report(vec![
        "Live Producer at 0x1, Refcount: 2.".to_owned(),
        "  Live Object at 0x2, Refcount: 1.".to_owned(),
        "D3D11 WARNING: something else".to_owned(),
    ]);
    assert_eq!(report.count, 2);
    assert_eq!(report.lines.len(), 2);
}
