use rune::{Key, Runtime};

use super::{press, press_char, rect, runtime_sized};
use crate::ui::fake_player::sim::{self, Media};
use crate::ui::fake_player::{self, Call};
use crate::ui::{ReviewState, Ui, death_entry, review_entry};

const TICK: f64 = 1.0 / 30.0;
const THUMB: f32 = 12.0;

struct Session {
    rt: Runtime<Ui>,
    clock: f64,
}

fn session(media: Media) -> Session {
    session_with(media, review_entry())
}

fn session_with(media: Media, entry: greyscreen_engine::RecordingEntry) -> Session {
    sim::simulate(media);
    let mut ui = Ui::preview();
    ui.settings.review_events = greyscreen_core::categories::CategorySet::all();
    ui.player_factory = sim::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.on_key = Some(Ui::on_key);
    rt.update(move |ui, _| ui.open_review_entry(entry));
    rt.frame(1.0 / 60.0);
    fake_player::clear_calls();
    let mut s = Session { rt, clock: 0.0 };
    s.run(TICK);
    s
}

fn windows_like(duration: f64) -> Media {
    Media {
        seek_latency: 0.2,
        layered: true,
        ..Media::new(duration)
    }
}

impl Session {
    fn run(&mut self, secs: f64) {
        for _ in 0..(secs / TICK).round() as usize {
            sim::advance(TICK);
            self.clock += TICK;
            let clock = self.clock;
            self.rt.update(move |ui, tx| {
                ui.poll(clock, tx);
            });
            self.rt.frame(TICK);
        }
    }

    fn review(&self) -> &ReviewState {
        self.rt.state.review.as_ref().expect("review")
    }

    fn time(&self) -> f64 {
        self.review().time()
    }

    fn playing(&self) -> bool {
        self.review().is_playing()
    }

    fn space(&mut self) {
        press(&mut self.rt, Key::Text(" ".into()));
    }

    fn drag_to(&mut self, fraction: f32) {
        let [x, y, w, h] = rect(&self.rt, "timeline");
        let (px, py) = (x + w * fraction, y + h / 2.0);
        self.rt.pointer_down(px, py);
        self.rt.pointer_up(px, py);
        self.rt.frame(TICK);
    }

    fn thumb_gap(&self) -> f32 {
        let fill = rect(&self.rt, "timeline-fill");
        let thumb = rect(&self.rt, "timeline-thumb");
        let track = rect(&self.rt, "timeline-track");
        let progress = (self.time() / self.review().duration()) as f32;
        let centre = thumb[0] + thumb[2] / 2.0;
        let expected = track[0] + THUMB / 2.0 + progress * (track[2] - THUMB);
        assert!(
            (centre - expected).abs() < 1.0,
            "thumb at {centre}, wanted {expected}"
        );
        assert!((fill[2] - progress * track[2]).abs() < 1.0, "fill {fill:?}");
        (centre - (fill[0] + fill[2])).abs()
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.05
}

#[test]
fn pausing_keeps_the_picture_and_the_time_where_they_were() {
    let mut s = session(windows_like(60.0));
    s.run(2.0);
    assert!(s.playing());
    let before = s.time();
    s.space();
    assert!(!s.playing());
    assert!(
        close(s.time(), before),
        "paused at {} from {before}",
        s.time()
    );
    s.run(1.0);
    assert!(close(s.time(), before), "settled at {}", s.time());
    assert!(close(sim::picture(), before), "picture {}", sim::picture());
}

#[test]
fn resuming_after_a_pause_carries_on_from_the_same_spot() {
    let mut s = session(windows_like(60.0));
    s.run(2.0);
    s.space();
    s.run(1.0);
    let paused = s.time();
    s.space();
    assert!(s.playing());
    s.run(1.0);
    let ahead = s.time() - paused;
    assert!((0.5..=1.05).contains(&ahead), "moved {ahead} from {paused}");
}

#[test]
fn rapid_play_pause_never_moves_backwards() {
    let mut s = session(windows_like(60.0));
    s.run(1.0);
    let mut last = s.time();
    for _ in 0..12 {
        s.space();
        s.run(TICK * 2.0);
        let now = s.time();
        assert!(now >= last - 1e-6, "went back from {last} to {now}");
        last = now;
    }
    assert!(s.playing(), "an even number of toggles keeps playing");
    s.run(1.0);
    assert!(s.time() > last + 0.5, "stuck at {}", s.time());
}

#[test]
fn seeking_while_playing_lands_and_plays_on() {
    let mut s = session(windows_like(60.0));
    s.run(1.0);
    s.drag_to(0.5);
    assert!(close(s.time(), 30.0), "jumped to {}", s.time());
    s.run(1.0);
    assert!(s.playing());
    assert!(s.time() > 30.5 && s.time() < 31.1, "at {}", s.time());
}

#[test]
fn a_seek_to_the_very_end_does_not_wedge_the_player() {
    let mut s = session(Media {
        pictures_end: 59.7,
        ..windows_like(60.0)
    });
    s.run(0.5);
    s.drag_to(0.999);
    s.run(1.0);
    assert!(sim::landed(), "the seek never landed");
    s.run(2.0);
    assert!(!s.playing(), "the clip should have ended");
    s.space();
    s.run(1.0);
    assert!(s.playing());
    assert!(
        s.time() < 2.0,
        "play after the end starts over, at {}",
        s.time()
    );
}

#[test]
fn play_at_the_end_starts_over() {
    let mut s = session(Media::new(3.0));
    s.run(4.0);
    assert!(!s.playing());
    assert!(close(s.time(), 3.0));
    s.space();
    s.run(0.5);
    assert!(s.playing());
    assert!(s.time() < 1.0, "at {}", s.time());
}

#[test]
fn play_while_a_seek_is_in_flight_plays_from_the_target() {
    let mut s = session(windows_like(60.0));
    s.run(1.0);
    s.space();
    s.run(0.5);
    s.drag_to(0.25);
    s.space();
    assert!(s.playing());
    s.run(1.0);
    assert!(s.time() > 15.5 && s.time() < 16.1, "at {}", s.time());
}

#[test]
fn the_thumb_sits_on_the_end_of_the_fill() {
    let mut s = session(Media::new(10.0));
    s.run(2.5);
    assert!(s.thumb_gap() < THUMB / 2.0, "playing gap {}", s.thumb_gap());
    s.space();
    s.run(0.5);
    assert!(s.thumb_gap() < THUMB / 2.0, "paused gap {}", s.thumb_gap());
    s.rt.set_viewport((1000.0, 680.0), 1.0);
    s.run(0.5);
    assert!(s.thumb_gap() < THUMB / 2.0, "resized gap {}", s.thumb_gap());
    press_char(&mut s.rt, "f");
    s.run(0.5);
    press_char(&mut s.rt, "f");
    s.run(0.5);
    assert!(
        s.thumb_gap() < THUMB / 2.0,
        "after theater gap {}",
        s.thumb_gap()
    );
}

#[test]
fn the_thumb_follows_the_fill_after_a_death_review() {
    let mut s = session_with(Media::new(60.0), death_entry());
    press_char(&mut s.rt, "d");
    s.run(1.0);
    press(&mut s.rt, Key::Escape);
    s.run(0.5);
    let review = s.review();
    assert!(review.death.is_none() || review.death_summary_open());
    if review.death_summary_open() {
        press(&mut s.rt, Key::Escape);
        s.run(0.5);
    }
    s.drag_to(0.8);
    s.run(0.5);
    assert!(s.thumb_gap() < THUMB / 2.0, "gap {}", s.thumb_gap());
}

#[test]
fn play_after_the_death_review_paused_replays_the_death() {
    let mut s = session_with(windows_like(60.0), death_entry());
    press_char(&mut s.rt, "d");
    s.run(0.5);
    let (start, end) = s.review().death_window().expect("window");
    s.run(end - start + 1.0);
    assert!(!s.playing(), "the review pauses at the end of the death");
    assert!(
        close(s.time(), end) || s.time() >= end - 0.1,
        "{}",
        s.time()
    );
    s.space();
    s.run(1.0);
    assert!(s.playing(), "play resumes");
    assert!(
        s.time() < end - 1.0,
        "replays from the start, at {}",
        s.time()
    );
}

#[test]
fn pause_only_pauses_the_player() {
    let mut s = session(windows_like(60.0));
    s.run(2.0);
    fake_player::clear_calls();
    s.space();
    assert_eq!(fake_player::calls(), vec![Call::Pause]);
}

#[test]
fn the_thumb_follows_the_fill_through_a_paused_resize() {
    let mut s = session(Media::new(10.0));
    s.run(5.0);
    s.space();
    s.run(0.5);
    for step in 0..24 {
        let width = 1400.0 - 20.0 * step as f32;
        s.rt.set_viewport((width, 680.0), 1.0);
        s.rt.frame(TICK);
    }
    for _ in 0..4 {
        s.rt.frame(TICK);
    }
    assert!(s.thumb_gap() < THUMB / 2.0, "gap {}", s.thumb_gap());
}

#[test]
fn the_thumb_covers_the_end_of_the_fill_on_every_frame_of_a_seek() {
    let mut s = session(Media::new(10.0));
    s.run(1.0);
    for key in ["marker-10", "marker-1", "marker-8"] {
        let [x, y, w, h] = rect(&s.rt, key);
        s.rt.pointer_down(x + w / 2.0, y + h / 2.0);
        s.rt.pointer_up(x + w / 2.0, y + h / 2.0);
        for _ in 0..12 {
            s.rt.frame(1.0 / 144.0);
            assert!(s.thumb_gap() <= THUMB / 2.0, "{key}: gap {}", s.thumb_gap());
        }
    }
}

#[test]
fn the_death_bar_thumb_covers_the_end_of_its_fill() {
    let mut s = session_with(Media::new(60.0), death_entry());
    press_char(&mut s.rt, "d");
    for _ in 0..4 {
        s.run(2.0);
        let track = rect(&s.rt, "death-track");
        let fill = rect(&s.rt, "death-fill");
        let thumb = rect(&s.rt, "death-thumb");
        let centre = thumb[0] + thumb[2] / 2.0;
        assert!(
            fill[2] > 0.0 && fill[2] <= track[2] + 0.5,
            "fill {fill:?} track {track:?}"
        );
        assert!(
            (centre - (fill[0] + fill[2])).abs() <= THUMB / 2.0,
            "thumb {centre} fill end {}",
            fill[0] + fill[2]
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn the_real_player_pauses_resumes_and_restarts_in_place() {
    let mut rt = runtime_sized(Ui::preview(), (1400.0, 680.0));
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    let mut s = Session { rt, clock: 0.0 };
    let run = |s: &mut Session, secs: f64| {
        for _ in 0..(secs / TICK).round() as usize {
            s.clock += TICK;
            let clock = s.clock;
            s.rt.update(move |ui, tx| {
                ui.poll(clock, tx);
            });
            s.rt.frame(TICK);
        }
    };
    run(&mut s, 2.0);
    assert!(s.playing());
    let before = s.time();
    assert!(before > 1.5, "played to {before}");
    s.space();
    run(&mut s, 1.0);
    assert!(!s.playing());
    assert!(close(s.time(), before), "held {} from {before}", s.time());
    s.space();
    run(&mut s, 1.0);
    assert!(
        (s.time() - before - 1.0).abs() < 0.1,
        "resumed to {}",
        s.time()
    );
    s.drag_to(0.5);
    run(&mut s, 0.5);
    let duration = s.review().duration();
    assert!(
        (s.time() - (duration * 0.5 + 0.5)).abs() < 0.15,
        "seek landed at {}",
        s.time()
    );
    run(&mut s, duration);
    assert!(!s.playing(), "ended");
    s.space();
    run(&mut s, 0.5);
    assert!(s.playing() && s.time() < 1.0, "restarted at {}", s.time());
}
