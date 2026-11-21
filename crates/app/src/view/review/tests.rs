#![allow(clippy::expect_used)]

use super::timeline::{drag_target, fraction};
use crate::ui::seek_target;

#[test]
fn markers_sit_at_their_share_of_the_duration() {
    assert_eq!(fraction(5.0, 10.0), 0.5);
    assert_eq!(fraction(0.0, 10.0), 0.0);
    assert_eq!(fraction(12.0, 10.0), 1.0);
    assert_eq!(fraction(5.0, 0.0), 0.0);
}

#[test]
fn seeking_to_a_marker_starts_five_seconds_early() {
    assert_eq!(seek_target(10.0), 5.0);
    assert_eq!(seek_target(1.0), 0.0);
}

#[test]
fn dragging_maps_the_pointer_to_a_time() {
    assert_eq!(drag_target(100.0, 400.0, 20.0), 5.0);
    assert_eq!(drag_target(-10.0, 400.0, 20.0), 0.0);
    assert_eq!(drag_target(500.0, 400.0, 20.0), 20.0);
    assert_eq!(drag_target(10.0, 0.0, 20.0), 0.0);
}

#[test]
fn event_titles_are_plain_words() {
    let entry = crate::ui::review_entry();
    let title = |id: u32| {
        let event = entry
            .meta
            .events
            .iter()
            .find(|e| e.id == id)
            .expect("event");
        super::panel::event_title(event)
    };
    assert_eq!(title(1), "Minions spawning");
    assert_eq!(title(3), "Kill");
    assert_eq!(title(5), "Death");
    assert_eq!(title(8), "Double kill");
    assert_eq!(title(10), "Inhibitor lost");
}
