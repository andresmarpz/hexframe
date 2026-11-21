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
    assert_eq!(effects(&mut phase, Event::Seek { to: 7.5 }), vec![]);
    assert_eq!(phase, Phase::Idle { at: 7.5 });
    assert_eq!(effects(&mut phase, Event::Open { play: false }), vec![Effect::Open { at: 7.5 }]);
}
