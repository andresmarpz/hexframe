use super::timeline::{drag_target, marker_x};
use crate::ui::seek_target;

#[test]
fn markers_sit_at_their_share_of_the_duration() {
    assert_eq!(marker_x(5.0, 10.0, 400.0), 200.0);
    assert_eq!(marker_x(0.0, 10.0, 400.0), 0.0);
    assert_eq!(marker_x(12.0, 10.0, 400.0), 400.0);
    assert_eq!(marker_x(5.0, 0.0, 400.0), 0.0);
}

#[test]
fn seeking_to_a_marker_starts_three_seconds_early() {
    assert_eq!(seek_target(10.0), 7.0);
    assert_eq!(seek_target(1.0), 0.0);
}

#[test]
fn dragging_maps_the_pointer_to_a_time() {
    assert_eq!(drag_target(100.0, 400.0, 20.0), 5.0);
    assert_eq!(drag_target(-10.0, 400.0, 20.0), 0.0);
    assert_eq!(drag_target(500.0, 400.0, 20.0), 20.0);
    assert_eq!(drag_target(10.0, 0.0, 20.0), 0.0);
}
