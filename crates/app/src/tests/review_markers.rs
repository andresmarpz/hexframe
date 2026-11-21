use rune::Runtime;

use super::{rect, runtime_sized};
use crate::ui::fake_player;
use crate::ui::{Ui, review_entry};

fn review_with(duration: f64, stale_width: f32) -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_sized(ui, (1280.0, 760.0));
    fake_player::set_duration(duration);
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
    rt.update(|ui, _| {
        if let Some(review) = &mut ui.review {
            review.track_width = stale_width;
        }
    });
    rt.frame(1.0 / 60.0);
    rt
}

fn marker_centers(rt: &Runtime<Ui>) -> Vec<(f64, f32)> {
    let events = rt
        .state
        .review
        .as_ref()
        .map(|r| r.entry.meta.events.clone())
        .unwrap_or_default();
    events
        .iter()
        .filter(|e| rt.find(&format!("marker-{}", e.id)).is_some())
        .map(|e| {
            let [x, _, w, _] = rect(rt, &format!("marker-{}", e.id));
            (e.rec_secs, x + w / 2.0)
        })
        .collect()
}

fn assert_spread(rt: &Runtime<Ui>, duration: f64) {
    let track = rect(rt, "timeline");
    let centers = marker_centers(rt);
    assert!(centers.len() > 3, "markers are drawn");
    let usable = track[2] - 14.0;
    for (secs, center) in centers {
        let expected = track[0] + 7.0 + (secs / duration).clamp(0.0, 1.0) as f32 * usable;
        assert!(
            (center - expected).abs() < 1.5,
            "marker at {secs}s sits at {center}, expected {expected}"
        );
    }
}

#[test]
fn markers_spread_over_the_track_even_with_a_stale_stored_width() {
    let rt = review_with(10.0, 37.0);
    assert_spread(&rt, 10.0);
}

#[test]
fn markers_spread_over_the_track_when_the_stored_width_is_zero() {
    let rt = review_with(10.0, 0.0);
    assert_spread(&rt, 10.0);
}

#[test]
fn markers_use_the_recording_length_when_the_player_reports_none() {
    let rt = review_with(0.0, 0.0);
    assert_spread(&rt, 10.0);
}
