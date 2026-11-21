use rune::Runtime;

use super::{rect, runtime_sized};
use crate::ui::fake_player;
use crate::ui::{Ui, review_entry};

fn review_at(width: f32, height: f32) -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_sized(ui, (width, height));
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
    rt
}

fn inside(window: (f32, f32), [x, y, w, h]: [f32; 4]) -> bool {
    x >= 0.0 && y >= 0.0 && x + w <= window.0 && y + h <= window.1
}

#[test]
fn the_panel_sits_below_the_player_in_a_narrow_window() {
    let rt = review_at(1000.0, 600.0);
    let player = rect(&rt, "review-player");
    let panel = rect(&rt, "review-panel");
    assert!(panel[1] >= player[1] + player[3]);
    assert_eq!(panel[0], player[0]);
    assert_eq!(panel[2], player[2]);
}

#[test]
fn the_fullscreen_button_fits_in_a_narrow_window() {
    let rt = review_at(1000.0, 600.0);
    assert!(inside((1000.0, 600.0), rect(&rt, "review-theater")));
    assert!(rt.find("review-speed").is_some());
    assert!(rt.find("review-mute").is_some());
}

#[test]
fn the_panel_sits_beside_the_player_in_a_wide_window() {
    let rt = review_at(1600.0, 900.0);
    let player = rect(&rt, "review-player");
    let panel = rect(&rt, "review-panel");
    assert!(panel[0] >= player[0] + player[2]);
    assert_eq!(panel[1], player[1]);
}
