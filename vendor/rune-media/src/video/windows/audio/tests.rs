use super::*;

const RATE: f64 = 1000.0;

/// One mono chunk of `len` samples at `pts`, each sample its own index.
fn feed(pts: f64, len: usize) -> Feed {
    let samples: Vec<f32> = (0..len).map(|i| i as f32).collect();
    let mut feed = Feed { gain: 1.0, samples: samples.len(), ..Feed::default() };
    feed.chunks.push_back(Chunk { pts, samples });
    feed
}

#[test]
fn the_first_write_starts_at_the_clock() {
    let mut f = feed(0.0, 1000);
    let mut out = [0.0; 10];
    assert_eq!(fill(&mut f, &mut out, 1, RATE, 0.1), None);
    assert_eq!(out[0], 100.0);
    assert_eq!(out[9], 109.0);
}

#[test]
fn a_write_close_to_the_clock_goes_on_without_a_skip_and_reports_the_drift() {
    let mut f = feed(0.0, 1000);
    let mut out = [0.0; 10];
    fill(&mut f, &mut out, 1, RATE, 0.1);
    let off = fill(&mut f, &mut out, 1, RATE, 0.105).unwrap();
    assert!((off - 0.005).abs() < 1e-9, "{off}");
    assert_eq!(out[0], 110.0);
}

#[test]
fn a_write_far_from_the_clock_jumps_to_it() {
    let mut f = feed(0.0, 1000);
    let mut out = [0.0; 10];
    fill(&mut f, &mut out, 1, RATE, 0.1);
    assert_eq!(fill(&mut f, &mut out, 1, RATE, 0.5), None);
    assert_eq!(out[0], 500.0);
}

#[test]
fn sound_not_decoded_yet_is_silence() {
    let mut f = feed(0.2, 1000);
    let mut out = [1.0; 10];
    fill(&mut f, &mut out, 1, RATE, 0.195);
    assert_eq!(out[..5], [0.0; 5]);
    assert_eq!(out[5], 0.0);
    assert_eq!(out[6], 1.0);
}

#[test]
fn a_nudge_moves_the_clock_the_sound_read() {
    let clock = Clock::default();
    let anchor = Anchor { at: Instant::now(), pos: 3.0, rate: 1.0 };
    clock.set(Some(anchor));
    clock.nudge(anchor, 0.01);
    assert_eq!(clock.get().map(|a| a.pos), Some(3.01));
}

#[test]
fn a_nudge_leaves_a_clock_set_again_alone() {
    let clock = Clock::default();
    let anchor = Anchor { at: Instant::now(), pos: 3.0, rate: 1.0 };
    clock.set(Some(Anchor { pos: 9.0, ..anchor }));
    clock.nudge(anchor, 0.01);
    assert_eq!(clock.get().map(|a| a.pos), Some(9.0));
    clock.set(None);
    clock.nudge(anchor, 0.01);
    assert_eq!(clock.get(), None);
}
