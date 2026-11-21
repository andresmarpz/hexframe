use super::*;
use crate::ui::Ui;

const LIGHTS_INSET: f32 = 76.0;

#[test]
fn only_the_rail_lets_the_traffic_lights_reach_into_the_page() {
    assert_eq!(lights_overhang(false, LIGHTS_INSET), 0.0);
    assert_eq!(
        lights_overhang(true, LIGHTS_INSET),
        LIGHTS_INSET + TRAFFIC_LIGHT_GAP - collapsed::WIDTH
    );
    assert_eq!(lights_overhang(true, 0.0), 0.0);
}

#[test]
fn narrow_windows_fold_the_nav_without_touching_the_setting() {
    let mut ui = Ui::preview();
    assert!(!is_folded(&ui, None));
    assert!(!is_folded(&ui, Some(FOLD_BELOW)));
    assert!(is_folded(&ui, Some(FOLD_BELOW - 1.0)));
    assert!(!ui.settings.sidebar_collapsed);
    ui.settings.sidebar_collapsed = true;
    assert!(is_folded(&ui, Some(2000.0)));
}
