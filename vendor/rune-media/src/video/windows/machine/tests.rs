use super::*;

fn effects(phase: &mut Phase, event: Event) -> Vec<Effect> {
    step(phase, event).into_iter().collect()
}

#[test]
fn a_playing_seek_past_the_last_picture_ends_where_it_was_asked_to_stand() {
    let mut phase = Phase::Buffering { at: 59.99, play: true };
    assert_eq!(effects(&mut phase, Event::Ended { at: 59.99 }), vec![]);
    assert_eq!(phase, Phase::Ended { at: 59.99 });
}

#[test]
fn a_paused_seek_past_the_last_picture_holds_there() {
    let mut phase = Phase::Buffering { at: 59.99, play: false };
    assert_eq!(effects(&mut phase, Event::Ended { at: 59.99 }), vec![]);
    assert_eq!(phase, Phase::Paused { at: 59.99 });
    let target = Instant::now();
    assert!(matches!(effects(&mut phase, Event::Play { target, lead: 0.0, rate: 1.0 }).as_slice(), [Effect::Clock(Some(_))]));
    assert!(matches!(phase, Phase::Playing { clock } if clock.pos == 59.99));
}

#[test]
fn a_seek_while_parked_keeps_the_phase_idle_at_the_target() {
    let mut phase = Phase::Idle { at: 3.0 };
    assert_eq!(effects(&mut phase, Event::Seek { to: 7.5, fast: false }), vec![]);
    assert_eq!(phase, Phase::Idle { at: 7.5 });
    assert_eq!(effects(&mut phase, Event::Open { play: false }), vec![Effect::Open { at: 7.5 }]);
}

#[test]
fn a_paused_clip_moves_its_hold_to_the_picture_it_shows() {
    let mut phase = Phase::Paused { at: 4.0 };
    assert_eq!(effects(&mut phase, Event::Pause { at: 4.016 }), vec![]);
    assert_eq!(phase, Phase::Paused { at: 4.016 });
}

#[test]
fn the_sound_moves_a_running_clock_at_its_rate_only() {
    let at = Instant::now();
    let mut phase = Phase::Playing { clock: Anchor { at, pos: 1.0, rate: 1.0 } };
    let heard = Anchor { at, pos: 1.004, rate: 1.0 };
    assert_eq!(effects(&mut phase, Event::Synced { clock: heard }), vec![]);
    assert_eq!(phase, Phase::Playing { clock: heard });
    let other = Anchor { at, pos: 1.0, rate: 2.0 };
    assert_eq!(effects(&mut phase, Event::Synced { clock: other }), vec![]);
    assert_eq!(phase, Phase::Playing { clock: heard });
}

#[test]
fn a_storm_of_seeks_while_playing_lands_on_the_last_and_plays_on() {
    let start = Instant::now();
    let mut phase = Phase::Playing { clock: Anchor { at: start, pos: 10.0, rate: 1.0 } };
    let mut asked = Vec::new();
    for i in 0..60 {
        let to = if i % 2 == 0 { 10.0 + f64::from(i) * 10.0 } else { 300.0 - f64::from(i) };
        for effect in effects(&mut phase, Event::Seek { to, fast: true }) {
            if let Effect::Seek { to, .. } = effect {
                asked.push(to);
            }
        }
    }
    assert_eq!(asked.len(), 60);
    assert_eq!(effects(&mut phase, Event::Seek { to: 600.0, fast: false }), vec![Effect::Seek { to: 600.0, fast: false }]);
    assert_eq!(phase, Phase::Buffering { at: 600.0, play: true });
    let target = start + Duration::from_millis(1100);
    assert!(matches!(effects(&mut phase, Event::Shown { pts: 600.0, target, lead: 0.0, rate: 1.0 }).as_slice(), [Effect::Clock(Some(_))]));
    assert!(matches!(phase, Phase::Playing { clock } if clock.pos == 600.0 && clock.at == target));
}

#[test]
fn buffering_that_outlives_its_bound_is_overdue_at_its_target() {
    let since = Instant::now();
    let phase = Phase::Buffering { at: 600.0, play: true };
    assert_eq!(overdue(phase, since, since + STUCK / 2), None);
    assert_eq!(overdue(phase, since, since + STUCK + Duration::from_millis(1)), Some(600.0));
    let playing = Phase::Playing { clock: Anchor { at: since, pos: 1.0, rate: 1.0 } };
    assert_eq!(overdue(playing, since, since + STUCK * 2), None);
}

#[test]
fn a_released_decoder_reopens_at_the_stuck_seek_and_keeps_playing() {
    let mut phase = Phase::Buffering { at: 600.0, play: true };
    assert_eq!(effects(&mut phase, Event::Released { at: 600.0 }), vec![Effect::Clock(None)]);
    assert_eq!(effects(&mut phase, Event::Open { play: true }), vec![Effect::Open { at: 600.0 }]);
    assert_eq!(effects(&mut phase, Event::Opened), vec![]);
    assert_eq!(phase, Phase::Buffering { at: 600.0, play: true });
}
