use super::*;
use crate::ui::Ui;

const LIGHTS_INSET: f32 = 76.0;

#[test]
fn only_the_rail_lets_the_traffic_lights_reach_into_the_page() {
    let mut ui = Ui::preview();
    assert_eq!(lights_overhang(&ui, LIGHTS_INSET), 0.0);
    ui.settings.sidebar_collapsed = true;
    assert_eq!(
        lights_overhang(&ui, LIGHTS_INSET),
        LIGHTS_INSET + TRAFFIC_LIGHT_GAP - collapsed::WIDTH
    );
    assert_eq!(lights_overhang(&ui, 0.0), 0.0);
}
