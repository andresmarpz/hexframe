use rune::Runtime;

use super::rect;
use super::runtime_sized;
use crate::ui::fake_player::sim::{self, Media};
use crate::ui::fake_player::{self, Call};
use crate::ui::{ReviewState, Ui, review_entry};

const TICK: f64 = 1.0 / 60.0;
const MOVES_PER_TICK: usize = 4;
const LENGTH: f64 = 1200.0;
const RESUME: f64 = 0.25;

struct Session {
    rt: Runtime<Ui>,
    clock: f64,
}

fn windows_like() -> Media {
    Media {
        seek_latency: 0.2,
        fast_latency: 0.03,
        layered: true,
        ..Media::new(LENGTH)
    }
}

fn session(media: Media) -> Session {
    sim::simulate(media);
    let mut ui = Ui::preview();
    ui.player_factory = sim::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    rt.frame(TICK);
    let mut s = Session { rt, clock: 0.0 };
    s.run(1.0);
    s
}

impl Session {
    fn tick(&mut self) {
        sim::advance(TICK);
        self.clock += TICK;
        let clock = self.clock;
        self.rt.update(move |ui, tx| {
            ui.poll(clock, tx);
        });
        self.rt.frame(TICK);
    }

    fn run(&mut self, secs: f64) {
        for _ in 0..(secs / TICK).round() as usize {
            self.tick();
        }
    }

    fn review(&self) -> &ReviewState {
        self.rt.state.review.as_ref().expect("review")
    }

    fn point(&self, fraction: f64) -> (f32, f32) {
        let [x, y, w, h] = rect(&self.rt, "timeline");
        (x + w * fraction as f32, y + h / 2.0)
    }

    /// Presses the thumb at `from`, moves the pointer four times a frame along `path` (0 to 1
    /// over `secs`), and leaves it held at the end.
    fn drag(&mut self, from: f64, secs: f64, path: impl Fn(f64) -> f64) {
        let (x, y) = self.point(from);
        self.rt.pointer_down(x, y);
        let ticks = (secs / TICK).round() as usize;
        for t in 0..ticks {
            for m in 0..MOVES_PER_TICK {
                let along = (t * MOVES_PER_TICK + m + 1) as f64 / (ticks * MOVES_PER_TICK) as f64;
                let (x, y) = self.point(path(along));
                self.rt.pointer_move(x, y);
            }
            self.tick();
            let shown = self.review().time() / LENGTH;
            assert!(
                (shown - path((t + 1) as f64 / ticks as f64)).abs() < 0.01,
                "the thumb lags the pointer: at {shown:.3}"
            );
        }
    }

    fn release(&mut self, at: f64) {
        let (x, y) = self.point(at);
        self.rt.pointer_up(x, y);
    }

    /// Seconds until the clip plays on from `from` with its picture there.
    fn resumes(&mut self, from: f64) -> f64 {
        let mut waited = 0.0;
        while !(sim::landed() && self.review().is_playing() && self.review().time() >= from - 1e-6)
        {
            assert!(waited < 30.0, "never played on from {from}");
            self.tick();
            waited += TICK;
        }
        waited
    }
}

fn seeks() -> usize {
    fake_player::seeks()
}

#[test]
fn a_two_second_drag_sends_a_few_seeks_and_plays_on_right_after_release() {
    let mut s = session(windows_like());
    assert!(s.review().is_playing());
    fake_player::clear_calls();
    s.drag(0.1, 2.0, |a| 0.1 + 0.5 * a);
    let during = seeks();
    s.release(0.6);
    let waited = s.resumes(0.6 * LENGTH);
    eprintln!(
        "{during} seeks during the drag, playing again {:.0} ms after release",
        waited * 1000.0
    );
    assert!(
        (5..=25).contains(&during),
        "{during} seeks during a 2 s drag"
    );
    assert!(
        waited <= RESUME,
        "playing again {:.0} ms after release",
        waited * 1000.0
    );
    assert_eq!(fake_player::calls().first(), Some(&Call::Pause));
    let exact: Vec<_> = fake_player::calls()
        .into_iter()
        .filter(|c| matches!(c, Call::Seek(_)))
        .collect();
    assert_eq!(exact.len(), 1, "exact seeks {exact:?}");
}

#[test]
fn a_back_and_forth_drag_plays_on_from_where_it_stops() {
    let mut s = session(windows_like());
    fake_player::clear_calls();
    s.drag(0.5, 1.0, |a| {
        if ((a * 20.0) as u32).is_multiple_of(2) {
            0.2
        } else {
            0.8
        }
    });
    let during = seeks();
    s.release(0.8);
    let waited = s.resumes(0.8 * LENGTH);
    assert!(during <= 13, "{during} seeks during a 1 s drag");
    assert!(
        waited <= RESUME,
        "playing again {:.0} ms after release",
        waited * 1000.0
    );
}

#[test]
fn a_drag_on_a_paused_clip_leaves_it_paused_on_the_exact_picture() {
    let mut s = session(windows_like());
    s.rt.update(|ui, _| {
        if let Some(review) = &mut ui.review {
            review.toggle();
        }
    });
    s.drag(0.2, 1.0, |a| 0.2 + 0.3 * a);
    s.release(0.5);
    s.run(0.5);
    assert!(!s.review().is_playing());
    assert!(sim::landed());
    assert!(
        (sim::picture() - 0.5 * LENGTH).abs() < 0.5,
        "picture {}",
        sim::picture()
    );
}

#[test]
fn a_pointer_held_still_gets_the_exact_picture_before_release() {
    let mut s = session(windows_like());
    s.drag(0.3, 0.5, |a| 0.3 + 0.1 * a);
    s.run(0.6);
    assert!(sim::landed());
    assert!(
        (sim::picture() - 0.4 * LENGTH).abs() < 0.5,
        "picture {}",
        sim::picture()
    );
    fake_player::clear_calls();
    s.release(0.4);
    s.tick();
    assert_eq!(fake_player::calls(), vec![Call::Play]);
    assert!(s.resumes(0.4 * LENGTH) <= RESUME);
}

#[test]
fn a_seek_that_never_lands_writes_one_stall_report() {
    let mut s = session(Media {
        seek_latency: 4.0,
        ..windows_like()
    });
    s.rt.update(|ui, _| ui.review_seek(600.0));
    s.run(2.0);
    let watch = &s.review().watch;
    let report = watch.last_report.clone().expect("a stall report");
    assert!(
        report.contains("playback stall: a seek waits for its picture at 600.000"),
        "{report}"
    );
    assert!(report.contains("seek 600.000"), "{report}");
    assert!(
        report.contains(&s.review().file.display().to_string()),
        "{report}"
    );
    s.run(3.0);
    assert_eq!(s.review().watch.reports, 1);
    assert!(s.review().is_playing());
}

#[test]
fn playback_that_runs_fast_writes_one_report() {
    let mut s = session(windows_like());
    for _ in 0..90 {
        sim::advance(2.0 * TICK);
        s.tick();
    }
    let watch = &s.review().watch;
    let report = watch.last_fast_report.clone().expect("a fast report");
    assert!(report.contains("playback ran fast"), "{report}");
    assert_eq!(watch.fast_reports, 1, "{report}");
    s.run(1.5);
    assert_eq!(s.review().watch.fast_reports, 1);
}

#[test]
fn a_jump_or_a_normal_run_writes_no_fast_report() {
    let mut s = session(windows_like());
    s.run(2.0);
    sim::advance(1.2);
    s.run(2.0);
    assert_eq!(s.review().watch.fast_reports, 0);
}
